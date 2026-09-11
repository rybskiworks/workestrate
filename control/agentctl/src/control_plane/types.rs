//! Backend-neutral resource names and the initial SSH management contract.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

/// A canonical opaque 256-bit identifier. It carries no backend address.
///
/// A launch ID is a public projection of a host-owned launch binding, not a
/// bearer credential. Knowing it never authenticates a caller.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct OpaqueId(String);

impl OpaqueId {
    /// Encode trusted identifier bytes without introducing another allocator.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(hex::encode(bytes))
    }

    /// Canonical wire representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for OpaqueId {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("expected a 64-character lowercase hexadecimal identifier");
        }
        Ok(Self(value))
    }
}

impl From<OpaqueId> for String {
    fn from(value: OpaqueId) -> Self {
        value.0
    }
}

/// Existing workload/context identity inside one configured state domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRef {
    pub context: Option<String>,
    pub name: String,
}

/// One instance of a workload, independent of how the backend names its VM.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanceRef {
    pub workload: WorkloadRef,
    pub instance: String,
}

/// One exact runtime launch. Replacement always changes `generation`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchRef {
    pub instance: InstanceRef,
    pub generation: OpaqueId,
}

/// Management authority, not SSH upstream authentication authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SshOperation {
    Inspect,
    Reconcile,
    Revoke,
}

/// Capability-specific actions share one resource permission table. There is
/// deliberately no wildcard action or implicit grant to future capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "capability", content = "action", rename_all = "snake_case")]
pub enum ControlOperation {
    Capabilities,
    SshCustody(SshOperation),
    GuestExec(ExecOperation),
    Lifecycle(LifecycleOperation),
}

impl From<SshOperation> for ControlOperation {
    fn from(operation: SshOperation) -> Self {
        Self::SshCustody(operation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecOperation {
    Start,
    Read,
    WriteStdin,
    Resize,
    Cancel,
    Release,
}

/// These operations require native launch fencing, not a name-based precheck.
/// Creation/replacement stays in the existing workload lifecycle implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleOperation {
    Inspect,
    Stop,
}

/// Process-local operation identity. The sequence is never reused within a
/// controller incarnation; knowing this value conveys no permission.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationRef {
    pub controller: OpaqueId,
    pub sequence: u64,
}

/// Guest literals only. No host environment, shell expansion, credentials or
/// unreviewed workload configuration are resolved by the dispatcher.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: std::collections::BTreeMap<String, String>,
    pub stdin: bool,
    pub tty: bool,
    pub timeout_ms: u32,
}

impl fmt::Debug for ExecCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Arguments and environment may contain sensitive caller data.
        f.debug_struct("ExecCommand")
            .field("stdin", &self.stdin)
            .field("tty", &self.tty)
            .field("timeout_ms", &self.timeout_ms)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecRequest {
    Start {
        command: ExecCommand,
    },
    Read {
        id: OperationRef,
        max_bytes: u32,
    },
    WriteStdin {
        id: OperationRef,
        bytes: Vec<u8>,
        eof: bool,
    },
    Resize {
        id: OperationRef,
        rows: u16,
        columns: u16,
    },
    Cancel {
        id: OperationRef,
    },
    Release {
        id: OperationRef,
    },
}

impl ExecRequest {
    pub fn operation(&self) -> ExecOperation {
        match self {
            Self::Start { .. } => ExecOperation::Start,
            Self::Read { .. } => ExecOperation::Read,
            Self::WriteStdin { .. } => ExecOperation::WriteStdin,
            Self::Resize { .. } => ExecOperation::Resize,
            Self::Cancel { .. } => ExecOperation::Cancel,
            Self::Release { .. } => ExecOperation::Release,
        }
    }

    pub fn id(&self) -> Option<&OperationRef> {
        match self {
            Self::Start { .. } => None,
            Self::Read { id, .. }
            | Self::WriteStdin { id, .. }
            | Self::Resize { id, .. }
            | Self::Cancel { id }
            | Self::Release { id } => Some(id),
        }
    }
}

/// One replaceable transport envelope for existing capability implementations.
/// It contains no asserted caller, backend address, raw SSH key or policy.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "capability",
    content = "request",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ControlRequest {
    Capabilities {
        launch: LaunchRef,
    },
    SshCustody(SshRequest),
    GuestExec {
        launch: LaunchRef,
        request: ExecRequest,
    },
    Lifecycle {
        launch: LaunchRef,
        operation: LifecycleOperation,
    },
}

impl ControlRequest {
    pub fn launch(&self) -> &LaunchRef {
        match self {
            Self::Capabilities { launch }
            | Self::GuestExec { launch, .. }
            | Self::Lifecycle { launch, .. } => launch,
            Self::SshCustody(request) => request.launch(),
        }
    }

    pub fn operation(&self) -> ControlOperation {
        match self {
            Self::Capabilities { .. } => ControlOperation::Capabilities,
            Self::SshCustody(request) => request.operation().into(),
            Self::GuestExec { request, .. } => ControlOperation::GuestExec(request.operation()),
            Self::Lifecycle { operation, .. } => ControlOperation::Lifecycle(*operation),
        }
    }
}

/// Implementation support, never evidence of guest/application readiness.
/// A backend lacking authoritative launch binding must report strict operations
/// unsupported even if it offers older name-addressed methods.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeCapabilities {
    pub launch_bound_exec: bool,
    pub stdin: bool,
    pub pty: bool,
    pub cancellation: bool,
    pub launch_bound_lifecycle: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecState {
    Accepted,
    Running,
    CancellationRequested,
    Exited {
        code: i32,
    },
    SpawnFailed,
    /// Transport loss or an interrupted control is not confirmed termination.
    Indeterminate,
}

impl ExecState {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Exited { .. } | Self::SpawnFailed)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExecEvent {
    Started,
    Stdout { bytes: Vec<u8> },
    Stderr { bytes: Vec<u8> },
    Exited { code: i32 },
    SpawnFailed,
    TransportLost,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecStatus {
    pub launch: LaunchRef,
    pub id: OperationRef,
    pub state: ExecState,
    pub events: Vec<ExecEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Running,
    Stopping,
    Stopped,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "capability",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ControlResponse {
    Capabilities {
        runtime: RuntimeCapabilities,
        ssh_custody: bool,
    },
    SshCustody(SshStatus),
    GuestExec(ExecStatus),
    Lifecycle {
        launch: LaunchRef,
        state: LifecycleState,
    },
}

/// Requests select registered resources and credential names, never raw keys,
/// arbitrary compiled policy, runtime addresses, or an asserted caller identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SshRequest {
    Inspect {
        launch: LaunchRef,
    },
    Reconcile {
        launch: LaunchRef,
        expected_revision: u64,
        credentials: BTreeSet<String>,
    },
    Revoke {
        launch: LaunchRef,
        expected_revision: u64,
    },
}

impl SshRequest {
    pub fn launch(&self) -> &LaunchRef {
        match self {
            Self::Inspect { launch }
            | Self::Reconcile { launch, .. }
            | Self::Revoke { launch, .. } => launch,
        }
    }

    pub fn operation(&self) -> SshOperation {
        match self {
            Self::Inspect { .. } => SshOperation::Inspect,
            Self::Reconcile { .. } => SshOperation::Reconcile,
            Self::Revoke { .. } => SshOperation::Revoke,
        }
    }
}

/// Both process incarnations bind observations after either side restarts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerSession {
    pub controller: OpaqueId,
    pub broker: OpaqueId,
    /// Fresh on every authenticated management connection, even to the same
    /// broker process, so buffered old acknowledgments cannot restore readiness.
    pub connection: u64,
}

/// Workestrate's current desired SSH policy for a registered launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredSshState {
    pub revision: u64,
    pub credentials: BTreeSet<String>,
    /// Destruction is terminal for this launch; policy revocation is not.
    pub destroyed: bool,
}

/// A broker report about one exact desired transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerObservation {
    pub session: BrokerSession,
    pub launch: LaunchRef,
    pub revision: u64,
    pub policy_digest: OpaqueId,
    pub outcome: AppliedOutcome,
    pub failure: Option<BrokerFailure>,
}

/// Safe diagnostic categories; peers never return raw credential material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrokerFailure {
    InvalidPolicy,
    MissingCredential,
    InvalidHostTrust,
    StaleLaunch,
    RetirementIncomplete,
    Unavailable,
}

/// Applied means the whole transition, including required session retirement,
/// finished. Receipt/acceptance alone must use `Pending`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedOutcome {
    Pending,
    Applied,
    Rejected,
    StateLost,
}

/// Non-secret desired and observed state returned by inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshStatus {
    pub launch: LaunchRef,
    pub desired: DesiredSshState,
    pub observed: Option<BrokerObservation>,
    pub ready: bool,
}

/// Bounded error categories. Transport errors must not expose secret payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlError {
    InvalidRequest,
    UnsupportedVersion,
    PermissionDenied,
    UnknownLaunch,
    StaleLaunch,
    RevisionConflict,
    InvalidPolicy,
    BrokerUnavailable,
    StateUnavailable,
    StaleObservation,
    RevisionExhausted,
    UnsupportedCapability,
    RuntimeUnavailable,
    UnknownOperation,
    OperationClosed,
    OperationIndeterminate,
    ResourceLimit,
    InvalidRuntimeResponse,
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidRequest => "control request is malformed",
            Self::UnsupportedVersion => "control protocol version is unsupported",
            Self::PermissionDenied => "control operation is not authorized",
            Self::UnknownLaunch => "launch is not registered",
            Self::StaleLaunch => "launch is no longer current",
            Self::RevisionConflict => "desired policy revision has changed",
            Self::InvalidPolicy => "requested policy is outside the compiled credential ceiling",
            Self::BrokerUnavailable => "broker state is not available",
            Self::StateUnavailable => "control state is not available",
            Self::StaleObservation => "broker report does not match current desired state",
            Self::RevisionExhausted => "policy revision cannot advance",
            Self::UnsupportedCapability => "required native capability is unsupported",
            Self::RuntimeUnavailable => "native runtime is unavailable",
            Self::UnknownOperation => "execution operation is not owned by this caller and launch",
            Self::OperationClosed => "execution operation is already terminal",
            Self::OperationIndeterminate => "execution outcome is not established",
            Self::ResourceLimit => "control resource limit exceeded",
            Self::InvalidRuntimeResponse => {
                "native response violates the bounded operation contract"
            }
        })
    }
}

impl std::error::Error for ControlError {}
