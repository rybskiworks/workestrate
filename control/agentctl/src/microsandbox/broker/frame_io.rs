//! Bounded receipt of local broker frames, without changing relay socket options.

use std::io::{self, Read};
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// One deadline covers both the length prefix and payload, including slow reads.
const FRAME_TIMEOUT: Duration = Duration::from_secs(10);
/// Check shutdown while waiting for bytes from an already accepted connection.
const CANCEL_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub(super) fn read_frame(
    stream: &mut UnixStream,
    max_bytes: usize,
    stop: Option<&AtomicBool>,
) -> io::Result<Vec<u8>> {
    read_frame_with_timeout(stream, max_bytes, stop, FRAME_TIMEOUT)
}

fn read_frame_with_timeout(
    stream: &mut UnixStream,
    max_bytes: usize,
    stop: Option<&AtomicBool>,
    timeout: Duration,
) -> io::Result<Vec<u8>> {
    let previous_timeout = stream.read_timeout()?;
    let deadline = Instant::now() + timeout;
    let result = (|| {
        let mut prefix = [0; 4];
        read_exact_until(stream, &mut prefix, deadline, stop)?;
        let length = u32::from_be_bytes(prefix) as usize;
        if length == 0 || length > max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("broker frame length {length} out of bounds (max {max_bytes})"),
            ));
        }
        let mut payload = vec![0; length];
        read_exact_until(stream, &mut payload, deadline, stop)?;
        Ok(payload)
    })();
    // A successful prelude hands this socket to an unbounded SSH relay. Do not
    // accidentally turn the frame deadline into an idle timeout on that relay.
    let restored = stream.set_read_timeout(previous_timeout);
    match result {
        Err(error) => Err(error),
        Ok(payload) => {
            restored?;
            Ok(payload)
        }
    }
}

fn read_exact_until(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    deadline: Instant,
    stop: Option<&AtomicBool>,
) -> io::Result<()> {
    while !bytes.is_empty() {
        if stop.is_some_and(|stop| stop.load(Ordering::Relaxed)) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "broker frame receipt cancelled",
            ));
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "broker frame receipt deadline exceeded",
            ));
        }
        stream.set_read_timeout(Some(remaining.min(CANCEL_POLL_INTERVAL)))?;
        match stream.read(bytes) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "broker frame ended before its declared length",
                ));
            }
            Ok(length) => bytes = &mut bytes[length..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::Interrupted
                        | io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error),
        }
    }
    if stop.is_some_and(|stop| stop.load(Ordering::Relaxed)) {
        return Err(io::Error::new(
            io::ErrorKind::Interrupted,
            "broker frame receipt cancelled",
        ));
    }
    if Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "broker frame receipt deadline exceeded",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, mpsc};

    // The peer stays open until receipt finishes, with an independent watchdog
    // so a broken read deadline fails the test instead of hanging the suite.
    fn receive_prefix(prefix: &[u8], max_bytes: usize) -> io::Result<Vec<u8>> {
        let (mut reader, mut peer) = UnixStream::pair().unwrap();
        reader
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        peer.write_all(prefix).unwrap();
        let (done, wait) = mpsc::channel::<()>();
        let watchdog = std::thread::spawn(move || {
            let _ = wait.recv_timeout(Duration::from_secs(2));
            drop(peer);
        });
        let result =
            read_frame_with_timeout(&mut reader, max_bytes, None, Duration::from_millis(10));
        assert_eq!(reader.read_timeout().unwrap(), Some(Duration::from_secs(3)));
        let _ = done.send(());
        watchdog.join().unwrap();
        result
    }

    #[test]
    fn every_incomplete_prefix_times_out_without_waiting_for_eof() {
        let payload = b"bounded-frame";
        let mut frame = (payload.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(payload);
        for length in 0..frame.len() {
            let error = receive_prefix(&frame[..length], payload.len()).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::TimedOut, "prefix {length}");
        }
    }

    #[test]
    fn empty_and_oversized_lengths_are_rejected_before_payload_allocation() {
        for length in [0_u32, 65, u32::MAX] {
            let error = receive_prefix(&length.to_be_bytes(), 64).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        }
    }

    #[test]
    fn successful_frame_preserves_relay_bytes_and_socket_timeout() {
        for previous in [None, Some(Duration::from_secs(3))] {
            let (mut reader, mut peer) = UnixStream::pair().unwrap();
            reader.set_read_timeout(previous).unwrap();
            peer.write_all(&[0, 0, 0, 3, 1, 2, 3, 9, 8]).unwrap();
            assert_eq!(read_frame(&mut reader, 3, None).unwrap(), [1, 2, 3]);
            assert_eq!(reader.read_timeout().unwrap(), previous);
            let mut relay = [0; 2];
            reader.read_exact(&mut relay).unwrap();
            assert_eq!(relay, [9, 8]);
        }
    }

    #[test]
    fn stop_cancels_a_stalled_payload_before_the_frame_deadline() {
        let (mut reader, mut peer) = UnixStream::pair().unwrap();
        peer.write_all(&[0, 0, 0, 3, 1]).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&stop);
        let (done, wait) = mpsc::channel::<()>();
        let watchdog = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            signal.store(true, Ordering::Relaxed);
            let _ = wait.recv_timeout(Duration::from_secs(2));
            drop(peer);
        });
        let error = read_frame(&mut reader, 3, Some(&stop)).unwrap_err();
        let _ = done.send(());
        watchdog.join().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(reader.read_timeout().unwrap(), None);
    }

    #[test]
    fn slow_progress_does_not_reset_the_whole_frame_deadline() {
        let (mut reader, mut peer) = UnixStream::pair().unwrap();
        let (done, wait) = mpsc::channel::<()>();
        let writer = std::thread::spawn(move || {
            // Each individual gap is shorter than the deadline, but the full
            // frame takes longer. A fresh timeout per read would accept it.
            for byte in [0, 0, 0, 3, 1, 2, 3] {
                if peer.write_all(&[byte]).is_err() {
                    break;
                }
                if wait.recv_timeout(Duration::from_millis(20)).is_ok() {
                    break;
                }
            }
            let _ = wait.recv_timeout(Duration::from_secs(2));
        });
        let error =
            read_frame_with_timeout(&mut reader, 3, None, Duration::from_millis(50)).unwrap_err();
        let _ = done.send(());
        drop(done);
        writer.join().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    }
}
