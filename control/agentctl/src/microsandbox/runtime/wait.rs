//! Host-side wait-for-port readiness probe (ADR 0026 addendum 2026-08-01).
//!
//! The W4 compose-mirrored dependency lifecycle starts service-kind
//! dependencies DETACHED, then must not let the dependent proceed until the
//! dep is actually accepting connections. This module provides the bounded
//! host-side TCP connect loop used for that: `TcpStream::connect_timeout`
//! against the dep's published host address on a 100ms poll interval until
//! `timeout` elapses.

use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use anyhow::Result;

/// Default readiness budget for a freshly-started service dependency (~15s,
/// ADR 0026 addendum 2026-08-01).
pub const DEFAULT_WAIT: Duration = Duration::from_secs(15);

/// Poll interval between connection attempts.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Block until a TCP connection to `ip:port` succeeds or `timeout` elapses.
///
/// Each attempt is itself bounded (one poll interval), so connection REFUSED
/// (the common not-yet-ready case) and a black-holed address both degrade to
/// the same poll loop. On exhaustion the error names the budget and the
/// address: `timed out after <n>s waiting for <ip>:<port>`.
pub fn wait_for_port(ip: IpAddr, port: u16, timeout: Duration) -> Result<()> {
    let addr = SocketAddr::new(ip, port);
    let deadline = Instant::now() + timeout;
    loop {
        match TcpStream::connect_timeout(&addr, POLL_INTERVAL) {
            Ok(_) => return Ok(()),
            Err(_) if Instant::now() < deadline => std::thread::sleep(POLL_INTERVAL),
            Err(_) => {
                anyhow::bail!(
                    "timed out after {}s waiting for {}:{}",
                    timeout.as_secs(),
                    ip,
                    port
                )
            }
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

    /// A live listener answers immediately: wait_for_port returns Ok well
    /// inside the budget. The listener is held for the duration of the call.
    #[test]
    fn bound_port_returns_ok_fast() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind fixture listener");
        let port = listener.local_addr().expect("local_addr").port();
        let start = Instant::now();
        wait_for_port(IpAddr::V4(Ipv4Addr::LOCALHOST), port, DEFAULT_WAIT)
            .expect("a listening port must be reachable");
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "a listening port must be reachable ~immediately"
        );
    }

    /// No listener on the port: the loop burns the FULL budget and then errs,
    /// naming the timeout and the address.
    #[test]
    fn unbound_port_times_out_with_address_in_error() {
        // Bind to grab a free port, then drop the listener so nothing is
        // listening on it.
        let port = {
            let listener =
                TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind probe listener");
            listener.local_addr().expect("local_addr").port()
        };
        let timeout = Duration::from_millis(300);
        let start = Instant::now();
        let err = wait_for_port(IpAddr::V4(Ipv4Addr::LOCALHOST), port, timeout)
            .expect_err("an unbound port must time out");
        let elapsed = start.elapsed();
        assert!(
            elapsed >= timeout,
            "must burn the full budget before failing; elapsed: {elapsed:?}"
        );
        assert!(
            elapsed < timeout + Duration::from_secs(2),
            "must not overshoot the budget materially; elapsed: {elapsed:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.starts_with("timed out after ") && msg.contains("waiting for 127.0.0.1:"),
            "error must name the timeout and address: {msg}"
        );
    }
}
