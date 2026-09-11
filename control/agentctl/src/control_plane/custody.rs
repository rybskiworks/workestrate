//! Desired SSH state and bidirectional applied-state reconciliation.
//!
//! A single control-service owner serializes these transitions. Runtime and
//! broker adapters perform I/O outside this model, then return verified events.
//! No broker report can create a workload or change its desired policy.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};

use super::authorization::{AuthenticatedCaller, CallerIdentity};
use super::types::{
    AppliedOutcome, BrokerObservation, BrokerSession, ControlError, DesiredSshState, InstanceRef,
    LaunchRef, OpaqueId, SshRequest, SshStatus,
};

#[cfg(unix)]
mod persistence;

/// Non-secret desired intent sent to the broker adapter for material resolution.
/// This is not sendable policy: the adapter must commit the effective policy's
/// shared-wire digest through `prepare_policy` before performing broker I/O.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshPolicyTransaction {
    pub session: BrokerSession,
    pub launch: LaunchRef,
    pub revision: u64,
    pub credentials: Vec<SshGrantPlan>,
    pub destroyed: bool,
}

/// Effective policy identity durably bound before an Apply can be transmitted.
/// Secret material remains owned by the resolving adapter, never this snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedSshPolicy {
    pub intent: SshPolicyTransaction,
    pub policy_digest: OpaqueId,
}

/// A mutation returns desired state, not a premature success/ready receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshChange {
    pub status: SshStatus,
    /// Absent when the broker is disconnected; desired state is still pending.
    pub transaction: Option<SshPolicyTransaction>,
}

#[derive(Debug)]
struct LaunchState {
    launch: LaunchRef,
    ceiling: BTreeMap<String, SshGrantPlan>,
    desired: DesiredSshState,
    effective_digest: Option<OpaqueId>,
    prepared_session: Option<BrokerSession>,
    observed: Option<BrokerObservation>,
    runtime_live: bool,
    ceiling_verified: bool,
}

/// One Workestrate state domain's SSH desired-state authority.
/// Construction does not discover, adopt or mutate a running backend.
#[derive(Debug)]
pub struct SshController {
    incarnation: OpaqueId,
    broker: Option<BrokerSession>,
    broker_connections: BTreeMap<OpaqueId, u64>,
    launches: BTreeMap<InstanceRef, LaunchState>,
    retired: BTreeSet<LaunchRef>,
    state_failed: Cell<bool>,
    #[cfg(unix)]
    store: Option<persistence::DesiredStore>,
}

impl SshController {
    pub fn new(incarnation: OpaqueId) -> Self {
        Self {
            incarnation,
            broker: None,
            broker_connections: BTreeMap::new(),
            launches: BTreeMap::new(),
            retired: BTreeSet::new(),
            state_failed: Cell::new(false),
            #[cfg(unix)]
            store: None,
        }
    }

    pub(crate) fn incarnation(&self) -> &OpaqueId {
        &self.incarnation
    }

    /// Check the complete trusted resource association independently of SSH
    /// readiness. A known backend ID cannot relabel another workload's launch.
    pub(crate) fn verify_registered_launch(&self, launch: &LaunchRef) -> Result<(), ControlError> {
        let state = self.current(launch)?;
        if state.desired.destroyed {
            return Err(ControlError::StaleLaunch);
        }
        if !state.runtime_live || !state.ceiling_verified {
            return Err(ControlError::RuntimeUnavailable);
        }
        Ok(())
    }

    pub(crate) fn ensure_state(&self) -> Result<(), ControlError> {
        #[cfg(unix)]
        if self
            .store
            .as_ref()
            .is_some_and(|store| store.check_current().is_err())
        {
            self.state_failed.set(true);
        }
        if self.state_failed.get() {
            Err(ControlError::StateUnavailable)
        } else {
            Ok(())
        }
    }

    /// A storage failure makes the owner unusable, including its old receipts.
    /// The service must close its direct broker connection on this error so
    /// broker admission is invalidated too. No transaction may be sent first.
    fn persist_desired(&mut self) -> Result<(), ControlError> {
        self.ensure_state()?;
        #[cfg(unix)]
        if let Some(store) = &self.store
            && store.save(self).is_err()
        {
            self.state_failed.set(true);
            self.broker = None;
            for state in self.launches.values_mut() {
                state.runtime_live = false;
                state.observed = None;
            }
            return Err(ControlError::StateUnavailable);
        }
        Ok(())
    }

    /// Register trusted compiled authority, never a delegated request body.
    /// Replacing a generation requires retirement of the previous launch.
    pub(crate) fn register_launch(
        &mut self,
        launch: LaunchRef,
        credentials: &CredentialsPlan,
        previous: Option<&LaunchRef>,
    ) -> Result<SshChange, ControlError> {
        self.ensure_state()?;
        if self.retired.contains(&launch) {
            return Err(ControlError::StaleLaunch);
        }
        let mut ceiling = BTreeMap::new();
        for record in &credentials.ssh {
            if record.name.is_empty()
                || ceiling
                    .insert(record.name.clone(), record.clone())
                    .is_some()
            {
                return Err(ControlError::InvalidPolicy);
            }
        }
        if let Some(old) = self.launches.get(&launch.instance) {
            if old.launch == launch {
                if old.desired.destroyed {
                    return Err(ControlError::StaleLaunch);
                }
                if old.ceiling != ceiling {
                    return Err(ControlError::InvalidPolicy);
                }
                self.current_mut(&launch)?.ceiling_verified = true;
                return self.change(&launch);
            }
            // Guest exec/lifecycle authority shares this launch association,
            // but a launch that never had broker custody must not depend on a
            // broker acknowledgement to be replaced. Revoked broker grants
            // still require retirement: inspect the ceiling, not selection.
            let had_custody = old
                .ceiling
                .values()
                .any(|record| record.binding == CredentialBinding::Broker);
            if previous != Some(&old.launch)
                || !old.desired.destroyed
                || (had_custody && !self.applied(old))
            {
                return Err(ControlError::RevisionConflict);
            }
            self.retired.insert(old.launch.clone());
        } else if previous.is_some() {
            return Err(ControlError::StaleLaunch);
        }
        let selected = ceiling
            .iter()
            .filter(|(_, record)| record.binding == CredentialBinding::Broker)
            .map(|(name, _)| name.clone())
            .collect();
        let state = LaunchState {
            launch: launch.clone(),
            ceiling,
            desired: DesiredSshState {
                revision: 1,
                credentials: selected,
                destroyed: false,
            },
            effective_digest: None,
            prepared_session: None,
            observed: None,
            runtime_live: false,
            ceiling_verified: true,
        };
        self.launches.insert(launch.instance.clone(), state);
        self.persist_desired()?;
        self.change(&launch)
    }

    /// Only the runtime adapter may assert a verified live launch binding.
    pub(crate) fn confirm_launch(&mut self, launch: &LaunchRef) -> Result<SshStatus, ControlError> {
        let state = self.current_mut(launch)?;
        if !state.ceiling_verified {
            return Err(ControlError::StateUnavailable);
        }
        if state.desired.destroyed {
            return Err(ControlError::StaleLaunch);
        }
        state.runtime_live = true;
        self.status(launch)
    }

    /// Destruction invalidates the exact generation before broker I/O starts.
    /// A delayed owner of an older generation cannot retire its replacement.
    pub(crate) fn retire_launch(&mut self, launch: &LaunchRef) -> Result<SshChange, ControlError> {
        let state = self.current_mut(launch)?;
        if !state.desired.destroyed {
            let next = next_revision(state.desired.revision)?;
            state.runtime_live = false;
            state.desired = DesiredSshState {
                revision: next,
                credentials: BTreeSet::new(),
                destroyed: true,
            };
            state.observed = None;
            state.effective_digest = None;
            state.prepared_session = None;
        }
        self.persist_desired()?;
        self.change(launch)
    }

    /// Handle an authenticated caller's request. Authentication is not inferred
    /// from a target resource, generation, UID string or credential name.
    pub fn request(
        &mut self,
        caller: &AuthenticatedCaller,
        request: SshRequest,
    ) -> Result<SshChange, ControlError> {
        caller.authorize(&request)?;
        self.ensure_state()?;
        if let CallerIdentity::WorkloadLaunch(origin) = caller.identity() {
            let live = self
                .current(origin)
                .is_ok_and(|state| state.runtime_live && !state.desired.destroyed);
            if !live {
                return Err(ControlError::PermissionDenied);
            }
        }
        let launch = request.launch().clone();
        match request {
            SshRequest::Inspect { .. } => {
                return Ok(SshChange {
                    status: self.status(&launch)?,
                    transaction: None,
                });
            }
            SshRequest::Reconcile {
                expected_revision,
                credentials,
                ..
            } => {
                let state = self.current_mut(&launch)?;
                if !state.ceiling_verified {
                    return Err(ControlError::StateUnavailable);
                }
                if state.desired.destroyed {
                    return Err(ControlError::StaleLaunch);
                }
                if expected_revision != state.desired.revision {
                    return Err(ControlError::RevisionConflict);
                }
                if credentials.iter().any(|name| {
                    !state
                        .ceiling
                        .get(name)
                        .is_some_and(|record| record.binding == CredentialBinding::Broker)
                }) {
                    return Err(ControlError::InvalidPolicy);
                }
                let next = next_revision(state.desired.revision)?;
                state.desired = DesiredSshState {
                    revision: next,
                    credentials,
                    destroyed: false,
                };
                state.observed = None;
                state.effective_digest = None;
                state.prepared_session = None;
            }
            SshRequest::Revoke {
                expected_revision, ..
            } => {
                let state = self.current_mut(&launch)?;
                if state.desired.destroyed {
                    return Err(ControlError::StaleLaunch);
                }
                if expected_revision != state.desired.revision {
                    return Err(ControlError::RevisionConflict);
                }
                let next = next_revision(state.desired.revision)?;
                state.desired = DesiredSshState {
                    revision: next,
                    credentials: BTreeSet::new(),
                    destroyed: false,
                };
                state.observed = None;
                state.effective_digest = None;
                state.prepared_session = None;
            }
        }
        self.persist_desired()?;
        self.change(&launch)
    }

    /// Retire observations before a replacement handshake can fence the old
    /// broker connection. Cancellation before Welcome must not leave old ready
    /// state behind; only a new authenticated Welcome can restore a session.
    pub(crate) fn begin_broker_connection(&mut self) -> Result<(), ControlError> {
        self.ensure_state()?;
        if let Some(session) = self.broker.clone() {
            self.broker_disconnected(&session)?;
        }
        Ok(())
    }

    /// Adopt the Welcome from the authenticated, launch-bound broker transport.
    /// The broker issues its incarnation and connection sequence; the controller
    /// must not invent a substitute that cannot fence the broker's own admission.
    /// Authentication is the adapter's responsibility, not a property of these IDs.
    pub(crate) fn broker_session_authenticated(
        &mut self,
        session: BrokerSession,
    ) -> Result<Vec<SshPolicyTransaction>, ControlError> {
        self.ensure_state()?;
        if session.controller != self.incarnation
            || session.broker == OpaqueId::from_bytes([0; 32])
            || session.connection == 0
            || self
                .broker_connections
                .get(&session.broker)
                .is_some_and(|previous| session.connection <= *previous)
        {
            return Err(ControlError::StaleObservation);
        }
        if !self.broker_connections.contains_key(&session.broker)
            && self.broker_connections.len() >= 256
        {
            return Err(ControlError::ResourceLimit);
        }
        self.broker_connections
            .insert(session.broker.clone(), session.connection);
        self.broker = Some(session);
        for state in self.launches.values_mut() {
            state.observed = None;
            state.prepared_session = None;
        }
        self.launches
            .values()
            .filter(|state| state.ceiling_verified || state.desired.destroyed)
            .map(|state| self.transaction(state))
            .collect()
    }

    /// Synthetic peer issuance for controller-only tests. Production callers
    /// must adopt the actual authenticated Welcome instead.
    #[cfg(test)]
    pub(crate) fn broker_connected(
        &mut self,
        broker: OpaqueId,
    ) -> Result<Vec<SshPolicyTransaction>, ControlError> {
        let connection = next_revision(*self.broker_connections.get(&broker).unwrap_or(&0))?;
        self.broker_session_authenticated(BrokerSession {
            controller: self.incarnation.clone(),
            broker,
            connection,
        })
    }

    /// Heartbeats require current ownership even before any launch is enrolled.
    /// A previously authenticated connection cannot renew after state loss.
    pub(crate) fn validate_broker_session(
        &self,
        session: &BrokerSession,
    ) -> Result<(), ControlError> {
        self.ensure_state()?;
        if self.broker.as_ref() != Some(session) {
            return Err(ControlError::StaleObservation);
        }
        Ok(())
    }

    /// A stale connection's disconnect cannot erase a newer broker session.
    pub(crate) fn broker_disconnected(
        &mut self,
        session: &BrokerSession,
    ) -> Result<(), ControlError> {
        self.ensure_state()?;
        if self.broker.as_ref() != Some(session) {
            return Err(ControlError::StaleObservation);
        }
        self.broker = None;
        for state in self.launches.values_mut() {
            state.observed = None;
            state.prepared_session = None;
        }
        Ok(())
    }

    /// Bind the digest calculated by the shared broker wire over the complete
    /// resolved policy, including key/trust versions and scanner material.
    /// Only the trusted material adapter calls this, never a delegated caller.
    /// A material-only rotation advances the desired revision even if credential
    /// names are unchanged. Failed persistence yields no sendable transaction.
    pub(crate) fn prepare_policy(
        &mut self,
        intent: &SshPolicyTransaction,
        policy_digest: OpaqueId,
    ) -> Result<PreparedSshPolicy, ControlError> {
        self.ensure_state()?;
        if self.broker.as_ref() != Some(&intent.session) {
            return Err(ControlError::StaleObservation);
        }
        if policy_digest == OpaqueId::from_bytes([0; 32]) {
            return Err(ControlError::InvalidPolicy);
        }
        let state = self.current(&intent.launch)?;
        if self.transaction(state)? != *intent {
            return Err(ControlError::RevisionConflict);
        }
        let state = self.current_mut(&intent.launch)?;
        if state.effective_digest.as_ref() != Some(&policy_digest) {
            if state.effective_digest.is_some() {
                state.desired.revision = next_revision(state.desired.revision)?;
            }
            state.effective_digest = Some(policy_digest.clone());
            state.observed = None;
        }
        // This session association is ephemeral. Reopening durable state or
        // reconnecting a broker requires fresh resolution/preparation again.
        state.prepared_session = Some(intent.session.clone());
        self.persist_desired()?;
        Ok(PreparedSshPolicy {
            intent: self.transaction(self.current(&intent.launch)?)?,
            policy_digest,
        })
    }

    /// Recheck immediately before a direct transport write or retry. Preparing
    /// material does not grant a reusable authority independent of later desired
    /// changes, launch replacement, broker reconnection or owner-state loss.
    pub(crate) fn validate_prepared(
        &self,
        prepared: &PreparedSshPolicy,
    ) -> Result<(), ControlError> {
        let state = self.current(&prepared.intent.launch)?;
        if self.broker.as_ref() != Some(&prepared.intent.session)
            || state.prepared_session.as_ref() != Some(&prepared.intent.session)
        {
            return Err(ControlError::StaleObservation);
        }
        if self.transaction(state)? != prepared.intent
            || state.effective_digest.as_ref() != Some(&prepared.policy_digest)
        {
            return Err(ControlError::RevisionConflict);
        }
        Ok(())
    }

    /// Accept applied state only for the exact current desired transaction.
    /// An unrequested report cannot register a launch or broaden its policy.
    pub(crate) fn observe(&mut self, report: BrokerObservation) -> Result<SshStatus, ControlError> {
        self.ensure_state()?;
        if self.broker.as_ref() != Some(&report.session) {
            return Err(ControlError::StaleObservation);
        }
        if (report.outcome == AppliedOutcome::Rejected) != report.failure.is_some() {
            return Err(ControlError::StaleObservation);
        }
        let state = self
            .current(&report.launch)
            .map_err(|_| ControlError::StaleObservation)?;
        if state.prepared_session.as_ref() != Some(&report.session)
            || report.revision != state.desired.revision
            || state.effective_digest.as_ref() != Some(&report.policy_digest)
        {
            return Err(ControlError::StaleObservation);
        }
        if report.outcome == AppliedOutcome::Pending
            && state
                .observed
                .as_ref()
                .is_some_and(|old| old.outcome == AppliedOutcome::Applied)
        {
            return Err(ControlError::StaleObservation);
        }
        let launch = report.launch.clone();
        if report.outcome == AppliedOutcome::StateLost {
            // Applied state is no longer authoritative on this connection.
            // Fence every receipt from it, including acknowledgments buffered
            // for other launches. Recovery needs a fresh authenticated session
            // and reassertion; replaying an old success cannot reopen access.
            self.broker_disconnected(&report.session)?;
        }
        self.current_mut(&launch)?.observed = Some(report);
        self.status(&launch)
    }

    fn current(&self, launch: &LaunchRef) -> Result<&LaunchState, ControlError> {
        self.ensure_state()?;
        let state = self
            .launches
            .get(&launch.instance)
            .ok_or(ControlError::UnknownLaunch)?;
        if state.launch != *launch {
            return Err(ControlError::StaleLaunch);
        }
        Ok(state)
    }

    fn current_mut(&mut self, launch: &LaunchRef) -> Result<&mut LaunchState, ControlError> {
        self.ensure_state()?;
        let state = self
            .launches
            .get_mut(&launch.instance)
            .ok_or(ControlError::UnknownLaunch)?;
        if state.launch != *launch {
            return Err(ControlError::StaleLaunch);
        }
        Ok(state)
    }

    fn transaction(&self, state: &LaunchState) -> Result<SshPolicyTransaction, ControlError> {
        self.ensure_state()?;
        if !state.ceiling_verified && !state.desired.destroyed {
            return Err(ControlError::StateUnavailable);
        }
        let session = self
            .broker
            .as_ref()
            .ok_or(ControlError::BrokerUnavailable)?
            .clone();
        let credentials: Vec<_> = state
            .desired
            .credentials
            .iter()
            .map(|name| {
                state
                    .ceiling
                    .get(name)
                    .cloned()
                    .ok_or(ControlError::InvalidPolicy)
            })
            .collect::<Result<_, _>>()?;
        Ok(SshPolicyTransaction {
            session,
            launch: state.launch.clone(),
            revision: state.desired.revision,
            credentials,
            destroyed: state.desired.destroyed,
        })
    }

    fn applied(&self, state: &LaunchState) -> bool {
        state.observed.as_ref().is_some_and(|report| {
            report.outcome == AppliedOutcome::Applied
                && self.broker.as_ref() == Some(&report.session)
                && report.launch == state.launch
                && report.revision == state.desired.revision
                && state.prepared_session.as_ref() == Some(&report.session)
                && state.effective_digest.as_ref() == Some(&report.policy_digest)
        })
    }

    fn status(&self, launch: &LaunchRef) -> Result<SshStatus, ControlError> {
        let state = self.current(launch)?;
        Ok(SshStatus {
            launch: state.launch.clone(),
            desired: state.desired.clone(),
            observed: state.observed.clone(),
            ready: state.runtime_live
                && state.ceiling_verified
                && !state.desired.destroyed
                && !state.desired.credentials.is_empty()
                && self.applied(state),
        })
    }

    fn change(&self, launch: &LaunchRef) -> Result<SshChange, ControlError> {
        let state = self.current(launch)?;
        let transaction =
            if self.broker.is_some() && (state.ceiling_verified || state.desired.destroyed) {
                Some(self.transaction(state)?)
            } else {
                None
            };
        Ok(SshChange {
            status: self.status(launch)?,
            transaction,
        })
    }
}

fn next_revision(revision: u64) -> Result<u64, ControlError> {
    revision
        .checked_add(1)
        .ok_or(ControlError::RevisionExhausted)
}
