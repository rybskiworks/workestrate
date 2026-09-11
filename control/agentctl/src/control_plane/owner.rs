//! One host endpoint for retained native operations and durable SSH intent.
//!
//! This uses the existing endpoint, controller and dispatcher. Workload plans
//! and retained native objects must already be supplied by the trusted command
//! entrypoint. An optional broker transport owns only Hello/Probe liveness;
//! without a complete Applied policy, custody remains unavailable. The owner
//! never stops an adopted VM when it exits.

use std::future::Future;
use std::io;
use std::os::fd::{AsFd, AsRawFd, OwnedFd};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio::time::Instant;

use super::authorization::AuthenticatedCaller;
use super::dispatcher::{ControlDispatcher, DrainSummary};
use super::local::{LocalAccess, LocalEndpoint, serve_connection};
use super::native::{MicrosandboxControl, NativeControl, ProtectedHostListener};
use super::types::{ControlError, ControlRequest, ControlResponse, LaunchRef};

#[path = "owner_broker.rs"]
mod broker;
use broker::{AttachedBroker, BrokerTransport};

const MAX_CLIENTS: usize = 16;
const QUEUE_CAPACITY: usize = 32;
const DRAIN_BUDGET: Duration = Duration::from_secs(10);
const BROKER_HEALTH_BUDGET: Duration = Duration::from_secs(5);

struct QueuedRequest {
    caller: AuthenticatedCaller,
    request: ControlRequest,
    reply: oneshot::Sender<Result<ControlResponse, ControlError>>,
    peer: OwnedFd,
}

pub(crate) struct HostControlOwner<N: NativeControl> {
    dispatcher: ControlDispatcher<N>,
    endpoint: Option<LocalEndpoint>,
    access: Arc<LocalAccess>,
    sender: mpsc::Sender<QueuedRequest>,
    receiver: mpsc::Receiver<QueuedRequest>,
    clients: JoinSet<io::Result<()>>,
    client_join_errors: usize,
    broker: Option<AttachedBroker>,
}

#[derive(Debug)]
pub(crate) struct OwnerExit {
    pub primary: io::Result<()>,
    pub drain: OwnerDrain,
}

#[derive(Debug)]
pub(crate) struct OwnerDrain {
    pub operations: DrainSummary,
    pub pending_clients: usize,
    pub client_join_errors: usize,
}

impl OwnerDrain {
    pub(crate) fn pending(&self) -> bool {
        !self.operations.pending.is_empty() || self.pending_clients != 0
    }

    pub(crate) fn complete(&self) -> bool {
        !self.pending() && self.operations.errors.is_empty() && self.client_join_errors == 0
    }
}

impl<N: NativeControl> HostControlOwner<N> {
    pub(crate) fn bind(
        directory: &Path,
        uid: u32,
        dispatcher: ControlDispatcher<N>,
    ) -> io::Result<Self> {
        dispatcher.check_owner().map_err(io::Error::other)?;
        let access = Arc::new(LocalAccess::new(uid));
        let endpoint = Some(LocalEndpoint::bind(directory, &access)?);
        let (sender, receiver) = mpsc::channel(QUEUE_CAPACITY);
        Ok(Self {
            dispatcher,
            endpoint,
            access,
            sender,
            receiver,
            clients: JoinSet::new(),
            client_join_errors: 0,
            broker: None,
        })
    }

    pub(crate) fn path(&self) -> io::Result<&Path> {
        self.endpoint
            .as_ref()
            .map(LocalEndpoint::path)
            .ok_or_else(closed)
    }

    fn fence(&mut self) {
        self.dispatcher.fence();
        self.receiver.close();
        self.endpoint.take();
        if let Some(broker) = &mut self.broker {
            broker.close(self.dispatcher.custody_mut());
        }
        while let Ok(request) = self.receiver.try_recv() {
            let _ = request.reply.send(Err(ControlError::StateUnavailable));
        }
        // Connection tasks only own their socket, decoder and queue wait.
        // Abort is a request; the JoinSet remains retained until actual joins.
        self.clients.abort_all();
    }

    fn check_current(&self) -> io::Result<()> {
        self.dispatcher.check_owner().map_err(io::Error::other)?;
        self.endpoint.as_ref().ok_or_else(closed)?.check_current()?;
        if let Some(broker) = &self.broker {
            broker.check_current().map_err(io::Error::other)?;
        }
        Ok(())
    }

    fn attachment_preflight(&mut self) -> io::Result<()> {
        if let Err(error) = self.check_current() {
            // Observing loss must also close an already attached idle link,
            // even when the caller never resumes the service loop afterward.
            self.fence();
            return Err(error);
        }
        if self.broker.is_some() {
            // A healthy duplicate request is not ownership loss. Preserve the
            // original transport and refuse to inspect or dial its replacement.
            return Err(io::Error::other("a broker resource is already retained"));
        }
        Ok(())
    }

    // Private resource seam: only the typed native attachment below is exposed
    // outside this module. Keep the resource on this owner before any await.
    async fn attach_broker_transport(
        &mut self,
        transport: Arc<dyn BrokerTransport>,
        deadline: Instant,
    ) -> io::Result<()> {
        self.attachment_preflight()?;
        let mut guard = AttachOnDrop {
            owner: self,
            armed: true,
        };
        guard.owner.broker = Some(AttachedBroker::new(transport));
        guard.owner.check_current()?;
        let endpoint = guard.owner.endpoint.as_ref().ok_or_else(closed)?;
        let broker = guard.owner.broker.as_mut().ok_or_else(closed)?;
        tokio::select! {
            lost = watch_endpoint(endpoint) => return Err(lost),
            result = broker.connect(guard.owner.dispatcher.custody_mut(), deadline) => {
                result.map_err(io::Error::other)?;
            },
        }
        guard.owner.check_current()?;
        guard.armed = false;
        Ok(())
    }

    async fn broker_health(&mut self) -> io::Result<()> {
        self.check_current()?;
        if let Some(broker) = &self.broker {
            let endpoint = self.endpoint.as_ref().ok_or_else(closed)?;
            tokio::select! {
                lost = watch_resources(endpoint, Some(broker)) => return Err(lost),
                result = broker.verify_running(Instant::now() + BROKER_HEALTH_BUDGET) => {
                    result.map_err(io::Error::other)?;
                },
            }
        }
        self.check_current()
    }

    async fn probe_broker(&mut self) -> io::Result<()> {
        self.check_current()?;
        let endpoint = self.endpoint.as_ref().ok_or_else(closed)?;
        let broker = self.broker.as_mut().ok_or_else(closed)?;
        tokio::select! {
            lost = watch_endpoint(endpoint) => return Err(lost),
            result = broker.probe(self.dispatcher.custody_mut()) => {
                result.map_err(io::Error::other)?;
            },
        }
        self.check_current()
    }

    /// Borrow this owner so deadline failure cannot silently release uncertain
    /// operation leases. A cancelled call fences immediately; callers can still
    /// use drain_until on this same object. Drop is only cancellation fallback.
    pub(crate) async fn serve_until<F>(&mut self, shutdown: F) -> OwnerExit
    where
        F: Future<Output = io::Result<()>>,
    {
        let guard = FenceOnDrop(self);
        let primary = guard.0.serve(shutdown).await;
        let drain = guard.0.drain_until(Instant::now() + DRAIN_BUDGET).await;
        OwnerExit { primary, drain }
    }

    pub(crate) async fn drain_until(&mut self, deadline: Instant) -> OwnerDrain {
        self.fence();
        let (operations, ()) = tokio::join!(
            self.dispatcher.drain_until(deadline),
            join_clients_until(&mut self.clients, &mut self.client_join_errors, deadline),
        );
        OwnerDrain {
            operations,
            pending_clients: self.clients.len(),
            client_join_errors: self.client_join_errors,
        }
    }

    fn accept_client(&mut self, stream: tokio::net::UnixStream) -> io::Result<()> {
        let peer = stream.as_fd().try_clone_to_owned()?;
        let access = Arc::clone(&self.access);
        let sender = self.sender.clone();
        self.clients.spawn(async move {
            serve_connection(stream, &access, |caller, request| async move {
                let (reply, receive) = oneshot::channel();
                sender
                    .try_send(QueuedRequest {
                        caller,
                        request,
                        reply,
                        peer,
                    })
                    .map_err(|error| match error {
                        mpsc::error::TrySendError::Full(_) => ControlError::ResourceLimit,
                        mpsc::error::TrySendError::Closed(_) => ControlError::StateUnavailable,
                    })?;
                receive.await.map_err(|_| ControlError::StateUnavailable)?
            })
            .await
        });
        Ok(())
    }

    async fn serve<F>(&mut self, shutdown: F) -> io::Result<()>
    where
        F: Future<Output = io::Result<()>>,
    {
        tokio::pin!(shutdown);
        let mut health = tokio::time::interval(Duration::from_secs(1));
        health.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            self.check_current()?;
            let probe_at = self.broker.as_ref().map(AttachedBroker::next_probe_at);
            // Do not let a continuously ready client queue starve a due Probe.
            // Already-issued native calls keep their existing bounded ownership
            // semantics; a late return cannot extend BrokerLink's absolute lease.
            if probe_at.is_some_and(|due| Instant::now() >= due) {
                tokio::select! {
                    biased;
                    stopped = &mut shutdown => return stopped,
                    result = self.probe_broker() => result?,
                }
                continue;
            }
            let endpoint = self.endpoint.as_ref().ok_or_else(closed)?;
            tokio::select! {
                stopped = &mut shutdown => return stopped,
                _ = wait_for_probe(probe_at) => {},
                _ = health.tick() => {
                    tokio::select! {
                        biased;
                        stopped = &mut shutdown => return stopped,
                        result = self.broker_health() => result?,
                    }
                },
                joined = self.clients.join_next(), if !self.clients.is_empty() => {
                    // Invalid or disconnected clients do not stop the owner;
                    // a task panic is an ownership failure, not normal input.
                    if joined.is_some_and(|result| result.is_err()) {
                        self.client_join_errors = self.client_join_errors.saturating_add(1);
                        return Err(io::Error::other("control connection task failed"));
                    }
                },
                stream = endpoint.accept(), if self.clients.len() < MAX_CLIENTS => {
                    self.accept_client(stream?)?;
                },
                queued = self.receiver.recv() => {
                    let queued = queued.ok_or_else(closed)?;
                    if queued.reply.is_closed() { continue; }
                    endpoint.check_current()?;
                    self.dispatcher.check_owner().map_err(io::Error::other)?;
                    // A normal client half-closes its writes, so read EOF is
                    // not cancellation. Query the retained socket immediately
                    // before dispatch; reactor readiness can be stale here.
                    if !client_connected(&queued.peer)? { continue; }
                    // Dropping dispatch preserves any already-issued operation
                    // reservation. Endpoint loss or shutdown never replays it.
                    let outcome = tokio::select! {
                        stopped = &mut shutdown => return stopped,
                        lost = watch_resources(endpoint, self.broker.as_ref()) => return Err(lost),
                        result = self.dispatcher.dispatch(&queued.caller, queued.request) => result,
                    };
                    self.check_current()?;
                    // This owner has no trusted material resolver yet. Consume
                    // only the response, leaving the original durable intent
                    // Pending. Hello/Probe never fabricate or apply a policy.
                    let response = outcome.map(|outcome| {
                        drop(outcome.custody_transaction);
                        outcome.response
                    });
                    let _ = queued.reply.send(response);
                },
            }
        }
    }
}

impl HostControlOwner<MicrosandboxControl> {
    /// Attach an already selected native launch and its once-captured route.
    /// Trusted bootstrap must supply both; this never creates, discovers,
    /// recaptures or reconnects a broker by name. No policy becomes Applied here.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "control serve does not yet select trusted broker bootstrap and route inputs"
        )
    )]
    pub(crate) async fn attach_broker(
        &mut self,
        launch: &LaunchRef,
        route: ProtectedHostListener,
        deadline: Instant,
    ) -> io::Result<()> {
        self.attachment_preflight()?;
        let transport = self
            .dispatcher
            .native()
            .retain_broker_route(launch, route)
            .map_err(io::Error::other)?;
        self.attach_broker_transport(transport, deadline).await
    }
}

struct AttachOnDrop<'a, N: NativeControl> {
    owner: &'a mut HostControlOwner<N>,
    armed: bool,
}

impl<N: NativeControl> Drop for AttachOnDrop<'_, N> {
    fn drop(&mut self) {
        if self.armed {
            self.owner.fence();
        }
    }
}

struct FenceOnDrop<'a, N: NativeControl>(&'a mut HostControlOwner<N>);

impl<N: NativeControl> Drop for FenceOnDrop<'_, N> {
    fn drop(&mut self) {
        self.0.fence();
    }
}

impl<N: NativeControl> Drop for HostControlOwner<N> {
    fn drop(&mut self) {
        self.fence();
    }
}

async fn watch_endpoint(endpoint: &LocalEndpoint) -> io::Error {
    watch_resources(endpoint, None).await
}

async fn watch_resources(endpoint: &LocalEndpoint, broker: Option<&AttachedBroker>) -> io::Error {
    loop {
        if let Err(error) = endpoint.check_current() {
            return error;
        }
        if let Some(broker) = broker
            && let Err(error) = broker.check_current()
        {
            return io::Error::other(error);
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn wait_for_probe(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn join_clients_until(
    clients: &mut JoinSet<io::Result<()>>,
    failures: &mut usize,
    deadline: Instant,
) {
    loop {
        match tokio::time::timeout_at(deadline, clients.join_next()).await {
            Ok(Some(Err(error))) if !error.is_cancelled() => *failures = failures.saturating_add(1),
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => return,
        }
    }
}

fn closed() -> io::Error {
    io::Error::other("host control owner is closed")
}

#[allow(unsafe_code)]
fn client_connected(peer: &OwnedFd) -> io::Result<bool> {
    let mut descriptor = libc::pollfd {
        fd: peer.as_raw_fd(),
        events: libc::POLLIN | libc::POLLOUT,
        revents: 0,
    };
    // SAFETY: one initialized pollfd is live for the call and the owned socket
    // cannot be closed/reused underneath this check. Zero timeout never waits.
    if unsafe { libc::poll(&mut descriptor, 1, 0) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(descriptor.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) == 0)
}

#[cfg(test)]
#[path = "owner_tests.rs"]
mod tests;
