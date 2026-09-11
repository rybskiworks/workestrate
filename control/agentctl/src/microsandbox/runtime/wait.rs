//! Cancellable host-side TCP readiness polling under a shared absolute deadline.
//! A successful connection establishes transport reachability, not application health.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use anyhow::Result;
use tokio::net::TcpStream;
use tokio::time::{Instant, sleep_until};

/// Default TCP readiness budget for dependency and batch service startup.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(15);

/// Maximum connection attempt duration and interval between attempts.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Wait until a TCP connection succeeds strictly before `deadline`.
///
/// The caller shares this deadline across registry lookup and all ports. No
/// connection or retry sleep receives a fresh budget. Dropping this future
/// cancels the pending socket or timer directly; there is no background worker.
pub async fn wait_for_port(ip: IpAddr, port: u16, deadline: Instant) -> Result<()> {
    let addr = SocketAddr::new(ip, port);
    loop {
        let now = Instant::now();
        if now >= deadline {
            anyhow::bail!("readiness deadline elapsed waiting for {addr}");
        }
        let attempt_deadline = deadline.min(now + POLL_INTERVAL);
        tokio::select! {
            biased;
            _ = sleep_until(attempt_deadline) => {}
            result = TcpStream::connect(addr) => {
                if result.is_ok() && Instant::now() < deadline {
                    return Ok(());
                }
            }
        }
        let now = Instant::now();
        if now < deadline {
            sleep_until(deadline.min(now + POLL_INTERVAL)).await;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, TcpListener};
    use tokio::time::{sleep, timeout};

    const LOCAL: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

    fn reserved_socket() -> tokio::net::TcpSocket {
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.bind(SocketAddr::new(LOCAL, 0)).unwrap();
        socket
    }

    #[tokio::test]
    async fn bound_port_returns_ok_fast() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let start = Instant::now();
        wait_for_port(LOCAL, port, start + DEFAULT_WAIT)
            .await
            .unwrap();
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn unbound_port_times_out_with_address_in_error() {
        let socket = reserved_socket();
        let port = socket.local_addr().unwrap().port();
        let budget = Duration::from_millis(150);
        let start = Instant::now();
        let error = wait_for_port(LOCAL, port, start + budget)
            .await
            .unwrap_err();
        assert!(start.elapsed() >= budget);
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(error.to_string().contains(&format!("127.0.0.1:{port}")));
    }

    #[tokio::test]
    async fn expired_deadline_never_connects_to_live_listener() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(wait_for_port(LOCAL, port, Instant::now()).await.is_err());
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    async fn cancelled_before_poll_does_not_connect() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let future = wait_for_port(
            LOCAL,
            listener.local_addr().unwrap().port(),
            Instant::now() + DEFAULT_WAIT,
        );
        drop(future);
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    async fn cancelled_retry_leaves_no_worker_or_later_connection() {
        let socket = reserved_socket();
        let port = socket.local_addr().unwrap().port();
        assert!(
            timeout(
                Duration::from_millis(25),
                wait_for_port(LOCAL, port, Instant::now() + DEFAULT_WAIT)
            )
            .await
            .is_err()
        );
        // The original wait has been dropped while retrying. A subsequently
        // opened listener must not receive an orphaned retry.
        let listener = socket.listen(16).unwrap();
        assert!(timeout(POLL_INTERVAL * 2, listener.accept()).await.is_err());
    }

    #[tokio::test]
    async fn delayed_listener_needs_sufficient_shared_budget() {
        let socket = reserved_socket();
        let port = socket.local_addr().unwrap().port();
        let started = Instant::now();
        // Both futures run on this task; no detached producer remains on failure.
        let delayed = async {
            sleep(Duration::from_millis(250)).await;
            let listener = socket.listen(16).unwrap();
            timeout(Duration::from_secs(2), listener.accept())
                .await
                .unwrap()
                .unwrap();
        };
        let probes = async {
            assert!(
                wait_for_port(LOCAL, port, started + Duration::from_millis(100))
                    .await
                    .is_err()
            );
            wait_for_port(LOCAL, port, started + Duration::from_secs(2))
                .await
                .unwrap();
        };
        tokio::join!(delayed, probes);
    }
}
