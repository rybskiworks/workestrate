//! Management permissions supplied by trusted connection adapters.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::types::{
    ControlError, ControlOperation, ControlRequest, LaunchRef, SshRequest, WorkloadRef,
};

/// An identity established by a transport adapter, not a request field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallerIdentity {
    LocalUid(u32),
    WorkloadLaunch(LaunchRef),
}

/// An exact resource scope; there are no prefix or implicit wildcard matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "scope",
    content = "resource",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ResourceScope {
    Workload(WorkloadRef),
    Launch(LaunchRef),
}

/// One server-owned permission. Operation and scope must match the same entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permission {
    pub resource: ResourceScope,
    pub operations: BTreeSet<ControlOperation>,
}

/// Authentication and permissions are attached outside the request decoder.
/// This deliberately has no deserializer or public constructor.
#[derive(Debug, Clone)]
pub struct AuthenticatedCaller {
    identity: CallerIdentity,
    permissions: Vec<Permission>,
    operator: bool,
}

impl AuthenticatedCaller {
    /// Only a kernel-authenticated local operator adapter uses this path.
    pub(crate) fn local_operator(uid: u32) -> Self {
        Self {
            identity: CallerIdentity::LocalUid(uid),
            permissions: Vec::new(),
            operator: true,
        }
    }

    /// The adapter obtains permissions from trusted configuration, not the peer.
    pub(crate) fn restricted(identity: CallerIdentity, permissions: Vec<Permission>) -> Self {
        Self {
            identity,
            permissions,
            operator: false,
        }
    }

    pub fn identity(&self) -> &CallerIdentity {
        &self.identity
    }

    /// The custody leaf uses the same action/resource matcher as dispatch.
    pub fn authorize(&self, request: &SshRequest) -> Result<(), ControlError> {
        self.authorize_action(request.operation().into(), request.launch())
    }

    pub fn authorize_control(&self, request: &ControlRequest) -> Result<(), ControlError> {
        self.authorize_action(request.operation(), request.launch())
    }

    fn authorize_action(
        &self,
        action: ControlOperation,
        launch: &LaunchRef,
    ) -> Result<(), ControlError> {
        // Exhaustive matching makes any future operator capability a deliberate
        // source change rather than an inherited wildcard permission.
        let operator = self.operator
            && match action {
                ControlOperation::Capabilities
                | ControlOperation::SshCustody(_)
                | ControlOperation::GuestExec(_)
                | ControlOperation::Lifecycle(_) => true,
            };
        if operator
            || self.permissions.iter().any(|permission| {
                permission.operations.contains(&action)
                    && match &permission.resource {
                        ResourceScope::Workload(workload) => workload == &launch.instance.workload,
                        ResourceScope::Launch(permitted) => permitted == launch,
                    }
            })
        {
            Ok(())
        } else {
            Err(ControlError::PermissionDenied)
        }
    }
}
