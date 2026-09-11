//! One retained native broker transport; the controller remains the authority.

use super::super::broker_link::BrokerLink;
use super::super::custody::SshController;
use super::super::native::{NativeFuture, RetainedBrokerRoute};
use super::*;
use tokio::net::UnixStream;

// This private resource seam also permits deterministic owner tests. Production
// construction accepts only RetainedBrokerRoute, not a supplied Unix path or an
// arbitrary callback claiming transport authentication.
pub(super) trait BrokerTransport: Send + Sync {
    fn check_current(&self) -> Result<(), ControlError>;
    fn verify_running(&self, deadline: Instant) -> NativeFuture<'_, ()>;
    fn connect(&self, deadline: Instant) -> NativeFuture<'_, UnixStream>;
}

impl BrokerTransport for RetainedBrokerRoute {
    fn check_current(&self) -> Result<(), ControlError> {
        RetainedBrokerRoute::check_current(self)
    }

    fn verify_running(&self, deadline: Instant) -> NativeFuture<'_, ()> {
        Box::pin(RetainedBrokerRoute::verify_running(self, deadline))
    }

    fn connect(&self, deadline: Instant) -> NativeFuture<'_, UnixStream> {
        Box::pin(RetainedBrokerRoute::connect(self, deadline))
    }
}

type RouteCheck = Box<dyn Fn() -> Result<(), ControlError> + Send + Sync>;
type ManagedLink = BrokerLink<UnixStream, RouteCheck>;

pub(super) struct AttachedBroker {
    transport: Arc<dyn BrokerTransport>,
    link: Option<ManagedLink>,
}

impl AttachedBroker {
    pub(super) fn new(transport: Arc<dyn BrokerTransport>) -> Self {
        Self {
            transport,
            link: None,
        }
    }

    pub(super) fn check_current(&self) -> Result<(), ControlError> {
        self.transport.check_current()
    }

    pub(super) fn next_probe_at(&self) -> Instant {
        self.link
            .as_ref()
            .map_or_else(Instant::now, BrokerLink::next_probe_at)
    }

    pub(super) async fn verify_running(&self, deadline: Instant) -> Result<(), ControlError> {
        open_deadline(deadline)?;
        tokio::time::timeout_at(deadline, self.transport.verify_running(deadline))
            .await
            .map_err(|_| ControlError::RuntimeUnavailable)??;
        open_deadline(deadline)
    }

    pub(super) async fn connect(
        &mut self,
        controller: &mut SshController,
        deadline: Instant,
    ) -> Result<(), ControlError> {
        open_deadline(deadline)?;
        self.verify_running(deadline).await?;
        let stream = tokio::time::timeout_at(deadline, self.transport.connect(deadline))
            .await
            .map_err(|_| ControlError::RuntimeUnavailable)??;
        self.check_current()?;
        controller.ensure_state()?;
        open_deadline(deadline)?;
        let transport = Arc::clone(&self.transport);
        let check: RouteCheck = Box::new(move || transport.check_current());
        let (link, pending) = tokio::time::timeout_at(
            deadline,
            BrokerLink::connect_verified(stream, check, controller),
        )
        .await
        .map_err(|_| ControlError::BrokerUnavailable)??;
        // Install ownership before the next await so a cancelled post-Hello
        // health check is closed by the host owner's synchronous fence.
        self.link = Some(link);
        self.verify_running(deadline).await?;
        controller.ensure_state()?;
        open_deadline(deadline)?;
        // Desired state remains in the one controller. This transport-only
        // increment does not resolve material, prepare policies or send Apply.
        // A Welcome is not authority to fabricate a policy for these intents.
        drop(pending);
        Ok(())
    }

    pub(super) async fn probe(
        &mut self,
        controller: &mut SshController,
    ) -> Result<(), ControlError> {
        self.verify_running(Instant::now() + BROKER_HEALTH_BUDGET)
            .await?;
        controller.ensure_state()?;
        self.link
            .as_mut()
            .ok_or(ControlError::BrokerUnavailable)?
            .probe(controller)
            .await?;
        // BrokerLink keeps its own conservative absolute lease and exchange
        // budget. A native health result cannot renew or extend either one.
        self.verify_running(Instant::now() + BROKER_HEALTH_BUDGET)
            .await?;
        controller.ensure_state()
    }

    pub(super) fn close(&mut self, controller: &mut SshController) {
        if let Some(mut link) = self.link.take() {
            link.close(controller);
        }
        // The original SDK/route capability remains retained until owner drop.
        // No stop/remove call or remote retirement assertion follows closure.
    }
}

fn open_deadline(deadline: Instant) -> Result<(), ControlError> {
    if Instant::now() >= deadline {
        Err(ControlError::BrokerUnavailable)
    } else {
        Ok(())
    }
}
