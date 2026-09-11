//! Explicit operator-owned control endpoint for selected running workloads.

use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use microsandbox::sandbox::SandboxStatus;
use tokio::time::{Instant, timeout_at};

use crate::control_plane::custody::SshController;
use crate::control_plane::dispatcher::ControlDispatcher;
use crate::control_plane::native::{MicrosandboxControl, NativeControl};
use crate::control_plane::owner::{HostControlOwner, OwnerDrain, OwnerExit};
use crate::control_plane::types::{InstanceRef, WorkloadRef};
use crate::microsandbox::plan::CredentialsPlan;
use crate::microsandbox::port_registry;
use crate::microsandbox::slots;
use crate::microsandbox::workload::{ConfigWorkload, Workload};

struct PreparedInstance {
    reference: InstanceRef,
    native_name: String,
    credentials: CredentialsPlan,
}

/// Resolve the active configuration and every selected registry association
/// before any asynchronous operation. There is no context switch, plan reload,
/// secret decryption, auto-start or caller-supplied policy ceiling here.
fn prepare_instances(instances: &[String]) -> Result<Vec<PreparedInstance>> {
    validate_selectors(instances)?;
    let context = crate::config::active_context_name();
    let state = crate::config::resolve_state_dir();
    let mut native_names = BTreeSet::new();
    let mut prepared = Vec::new();
    for instance in instances {
        let record = port_registry::find_record(&state, instance)?
            .context("selected instance has no readable registry record")?;
        ensure!(
            record.instance == *instance
                && record.context == context
                && slots::context_consistent_with_instance(
                    instance,
                    &record.workload,
                    context.as_deref()
                ),
            "selected instance does not belong to the active workload context"
        );
        let id = slots::instance_id_of(instance);
        if let Some(id) = id {
            slots::validate_instance_id(id)?;
        }
        let workload =
            ConfigWorkload::new_with_use_overrides_and_instance(&record.workload, &[], id)?;
        ensure!(
            workload.namespace() == record.namespace,
            "selected instance is from another configuration namespace"
        );
        let plan = workload.plan();
        let native_name = slots::msb_name_of_instance(instance);
        ensure!(
            native_names.insert(native_name.clone()),
            "selected native names collide"
        );
        prepared.push(PreparedInstance {
            reference: InstanceRef {
                workload: WorkloadRef {
                    context: context.clone(),
                    name: record.workload,
                },
                instance: instance.clone(),
            },
            native_name,
            credentials: plan.credentials.unwrap_or(CredentialsPlan {
                ssh: Vec::new(),
                signing: Vec::new(),
                strict: false,
                strict_origin: None,
            }),
        });
    }
    ensure!(
        crate::config::active_context_name() == context,
        "active context changed while preparing control plans"
    );
    Ok(prepared)
}

fn validate_selectors(instances: &[String]) -> Result<()> {
    ensure!(
        !instances.is_empty() && instances.len() <= 128,
        "select between one and 128 instances"
    );
    let mut unique = BTreeSet::new();
    for instance in instances {
        ensure!(
            instance.len() <= 256
                && instance.starts_with(|c: char| c.is_ascii_alphanumeric())
                && instance
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"._-@".contains(&byte)),
            "invalid instance selector"
        );
        ensure!(unique.insert(instance), "duplicate instance selector");
    }
    Ok(())
}

/// One explicit host endpoint for selected, already-running workload instances.
/// `state_dir` is private control state, not a replacement runtime home. The
/// caller must supply an existing canonical private directory; --initialize
/// creates a new desired store beneath it, while reopening never creates one.
pub async fn cmd_control_serve(
    state_dir: &Path,
    instances: &[String],
    initialize: bool,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let mut signals = ShutdownSignals::new()?;
    let prepared = prepare_instances(instances)?;
    // SAFETY: geteuid has no pointer arguments or memory-safety preconditions.
    #[allow(unsafe_code)]
    let uid = unsafe { libc::geteuid() };
    let metadata = std::fs::symlink_metadata(state_dir)?;
    ensure!(
        state_dir.is_absolute()
            && std::fs::canonicalize(state_dir)? == state_dir
            && metadata.is_dir()
            && metadata.uid() == uid
            && metadata.mode() & 0o077 == 0,
        "control state directory must be canonical, private and operator-owned"
    );
    let desired = state_dir.join("desired");
    let mut controller = if initialize {
        SshController::initialize(&desired, uid)?
    } else {
        SshController::open(&desired, uid)?
    };
    let backend = microsandbox::backend::default_backend();
    ensure!(
        backend.as_local().is_some(),
        "strict host control requires the local native backend"
    );
    let mut native = MicrosandboxControl::new(backend.clone());
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut launches = Vec::new();
    // Backend selection is captured once. All get/connect operations use that
    // same Arc, not repeated ambient-default lookup or connect-or-start.
    for selected in prepared {
        let handle = timeout_at(
            deadline,
            backend
                .sandboxes()
                .get(backend.clone(), &selected.native_name),
        )
        .await
        .context("native selection deadline")??;
        ensure!(
            handle.status_snapshot() == SandboxStatus::Running,
            "selected instance is not running"
        );
        let sandbox = timeout_at(deadline, handle.connect())
            .await
            .context("native connection deadline")??;
        ensure!(
            timeout_at(deadline, sandbox.status())
                .await
                .context("native observation deadline")??
                == SandboxStatus::Running,
            "selected launch is no longer running"
        );
        let launch = timeout_at(
            deadline,
            native.retain(selected.reference, &selected.native_name, sandbox),
        )
        .await
        .context("native retention deadline")??;
        let change = controller.register_launch(launch.clone(), &selected.credentials, None)?;
        ensure!(
            change.transaction.is_none(),
            "unattached control owner unexpectedly prepared broker I/O"
        );
        let status = controller.confirm_launch(&launch)?;
        ensure!(
            !status.ready,
            "unattached broker cannot establish custody readiness"
        );
        launches.push(launch);
    }
    let dispatcher = ControlDispatcher::with_custody(native, controller);
    let mut owner = HostControlOwner::bind(&state_dir.join("endpoint"), uid, dispatcher)?;
    println!(
        "{}",
        serde_json::json!({"endpoint": owner.path()?, "launches": launches,
        "broker": "unavailable", "custody_ready": false})
    );
    let OwnerExit { primary, drain } = owner.serve_until(signals.next()).await;
    let (drain, cleanup_error) =
        retain_until_retired(&mut owner, drain, async || signals.next().await).await;
    if primary.is_err() || cleanup_error {
        anyhow::bail!(
            "host control primary outcome: {primary:?}; final cleanup: {drain:?}; \
            an earlier cleanup error is preserved: {cleanup_error}"
        );
    }
    Ok(())
}

pub(crate) async fn retain_until_retired<N, F>(
    owner: &mut HostControlOwner<N>,
    mut drain: OwnerDrain,
    mut next_signal: F,
) -> (OwnerDrain, bool)
where
    N: NativeControl,
    F: AsyncFnMut() -> std::io::Result<()>,
{
    // Summary contains public identities/categories only, never command or
    // diagnostic byte payloads. Preserve both primary and cleanup failures.
    let mut cleanup_error = !drain.complete();
    while drain.pending() {
        eprintln!(
            "host control remains fenced; cleanup retained: {drain:?}. \
            Ctrl-C or SIGTERM retries only these original leases for up to ten seconds; \
            it never resumes admission, replays commands or stops adopted VMs. \
            Persistent unconfirmed termination may require operator escalation; \
            forced exit is not clean retirement."
        );
        if next_signal().await.is_err() {
            // Losing the shutdown signal source is not proof that outstanding
            // native sessions retired. Keep this same fenced owner alive until
            // explicit operator escalation; never silently drop its leases.
            eprintln!(
                "shutdown signal source closed; retained cleanup requires operator escalation; forced exit is not clean retirement"
            );
            std::future::pending::<()>().await;
        }
        // Tokio retains notifications during this drain (repeated signals may
        // coalesce). They are consumed only for a later retry, never to replace
        // or extend this absolute deadline.
        drain = owner
            .drain_until(Instant::now() + Duration::from_secs(10))
            .await;
        cleanup_error |= !drain.complete();
    }
    (drain, cleanup_error)
}

struct ShutdownSignals {
    interrupt: tokio::signal::unix::Signal,
    terminate: tokio::signal::unix::Signal,
}

impl ShutdownSignals {
    fn new() -> std::io::Result<Self> {
        use tokio::signal::unix::{SignalKind, signal};
        Ok(Self {
            interrupt: signal(SignalKind::interrupt())?,
            terminate: signal(SignalKind::terminate())?,
        })
    }

    async fn next(&mut self) -> std::io::Result<()> {
        let received = tokio::select! {
            signal = self.interrupt.recv() => signal,
            signal = self.terminate.recv() => signal,
        };
        received.ok_or_else(|| std::io::Error::other("shutdown signal stream closed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_selectors_are_bounded_unique_and_not_filesystem_paths() {
        assert!(
            validate_selectors(&["context-worker@one".into(), "context-worker@two".into()]).is_ok()
        );
        for invalid in [
            "",
            "../worker",
            "/worker",
            ".worker",
            "worker/child",
            "worker\n",
            "worker\0",
            "wørker",
        ] {
            assert!(validate_selectors(&[invalid.into()]).is_err());
        }
        assert!(validate_selectors(&[]).is_err());
        assert!(validate_selectors(&["worker".into(), "worker".into()]).is_err());
        assert!(validate_selectors(&["x".repeat(257)]).is_err());
        assert!(
            validate_selectors(
                &(0..129)
                    .map(|id| format!("worker-{id}"))
                    .collect::<Vec<_>>()
            )
            .is_err()
        );
    }
}
