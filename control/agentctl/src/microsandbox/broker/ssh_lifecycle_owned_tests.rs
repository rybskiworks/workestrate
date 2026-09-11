//! Disposable filesystem and owned-thread retirement controls; no VM or TCP.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::time::Duration;

fn fixture(
    work: impl FnOnce(Arc<AtomicBool>) -> std::io::Result<()> + Send + 'static,
) -> (tempfile::TempDir, SshShimHandle) {
    let dir = tempfile::tempdir().unwrap();
    let registry = Arc::new(CidRegistry::open(dir.path()).unwrap());
    let epoch = EpochToken::issue("fixture", 0).unwrap();
    let cid = registry.allocate("fixture", &epoch).unwrap();
    let socket_path = dir.path().join("owned.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
    let metadata = std::fs::symlink_metadata(&socket_path).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&stop);
    let finalize_requested = Arc::new(AtomicBool::new(false));
    let finalize = Arc::clone(&finalize_requested);
    let owner_dropped = Arc::new(AtomicBool::new(false));
    let dropped = Arc::clone(&owner_dropped);
    let loop_path = socket_path.clone();
    let join = std::thread::spawn(move || {
        work(signal)?;
        drop(listener);
        if await_finalization_intent(&finalize, &dropped) {
            finalize_shim(
                &registry,
                cid,
                "fixture",
                &epoch,
                &loop_path,
                (metadata.dev(), metadata.ino()),
            )
        } else {
            Ok(ShimExit::CancelledWithoutFinalization)
        }
    });
    (
        dir,
        SshShimHandle {
            socket_path,
            transport_cid: u64::from(cid),
            instance: "fixture".into(),
            cid,
            stop,
            finalize_requested,
            owner_dropped,
            join: Some(join),
            retirement_failure: None,
            retired: false,
        },
    )
}

fn assert_reserved(dir: &Path, handle: &SshShimHandle) {
    assert!(handle.socket_path.exists());
    assert!(
        CidRegistry::open(dir)
            .unwrap()
            .lookup(handle.cid)
            .unwrap()
            .is_some()
    );
    assert!(!handle.retired);
}

#[tokio::test]
async fn fenced_retained_owner_preserves_reservation_until_explicit_retirement() {
    let (release, wait) = std::sync::mpsc::channel();
    let (done, joined_workers) = tokio::sync::oneshot::channel();
    let (dir, mut handle) = fixture(move |stop| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(stop.load(Ordering::Acquire));
        done.send(()).unwrap();
        Ok(())
    });
    let reserved = ReservationSnapshot::new(dir.path(), &handle);
    handle.fence();
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), joined_workers)
        .await
        .unwrap()
        .unwrap();
    // This is the stop-failure posture: joined host relays do not establish
    // stopped guest state, so the runtime owner must not authorize release.
    assert!(!handle.finalize_requested.load(Ordering::Acquire));
    assert!(!handle.owner_dropped.load(Ordering::Acquire));
    reserved.assert_unchanged(&handle);
    assert!(!handle.join.as_ref().unwrap().is_finished());
    // A later confirmed stop permits the same retained thread to finalize.
    handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
        .await
        .unwrap();
    assert!(handle.retired);
    assert!(handle.join.is_none());
    assert!(!handle.socket_path.exists());
    assert!(
        CidRegistry::open(dir.path())
            .unwrap()
            .lookup(handle.cid)
            .unwrap()
            .is_none()
    );
}

#[test]
fn fenced_owner_drop_preserves_reservation_after_actual_thread_join() {
    let (release, wait) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |stop| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(stop.load(Ordering::Acquire));
        Ok(())
    });
    let reserved = ReservationSnapshot::new(dir.path(), &handle);
    let path = handle.socket_path.clone();
    let cid = handle.cid;
    let finalize = handle.finalize_requested.clone();
    let dropped = handle.owner_dropped.clone();
    handle.fence();
    let join = handle.join.take().unwrap();
    // Keep this join only as a test observer; Drop does not claim completion.
    drop(handle);
    assert!(dropped.load(Ordering::Acquire));
    assert!(!finalize.load(Ordering::Acquire));
    release.send(()).unwrap();
    assert!(matches!(
        join.join().unwrap().unwrap(),
        ShimExit::CancelledWithoutFinalization
    ));
    assert_eq!(std::fs::read(reserved.entry_path).unwrap(), reserved.bytes);
    let socket = std::fs::symlink_metadata(path).unwrap();
    assert_eq!((socket.dev(), socket.ino()), reserved.socket_identity);
    assert!(
        CidRegistry::open(dir.path())
            .unwrap()
            .lookup(cid)
            .unwrap()
            .is_some()
    );
}

// Read only the exact synthetic record while intentionally holding its registry
// lock. Reopening CidRegistry would try to acquire that same lock recursively.
struct ReservationSnapshot {
    entry_path: PathBuf,
    bytes: Vec<u8>,
    socket_identity: (u64, u64),
}

impl ReservationSnapshot {
    fn new(dir: &Path, handle: &SshShimHandle) -> Self {
        let entry_path = dir
            .join("var/run/broker")
            .join(format!("cid-{}.json", handle.cid));
        let bytes = std::fs::read(&entry_path).unwrap();
        assert!(bytes.len() < 4096);
        let entry: crate::microsandbox::broker::registry::CidEntry =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(entry.cid, handle.cid);
        assert_eq!(entry.instance, handle.instance);
        assert!(entry.expired_at_secs.is_none());
        let socket = std::fs::symlink_metadata(&handle.socket_path).unwrap();
        Self {
            entry_path,
            bytes,
            socket_identity: (socket.dev(), socket.ino()),
        }
    }

    fn assert_unchanged(&self, handle: &SshShimHandle) {
        // Full bytes include the exact original epoch, not merely the CID.
        assert_eq!(std::fs::read(&self.entry_path).unwrap(), self.bytes);
        let socket = std::fs::symlink_metadata(&handle.socket_path).unwrap();
        assert_eq!((socket.dev(), socket.ino()), self.socket_identity);
        assert!(!handle.retired);
    }
}

#[tokio::test]
async fn retirement_deadline_retains_worker_and_reservation_then_retry_joins() {
    let (release, wait) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |_| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(())
    });
    let error = handle
        .retire_until(tokio::time::Instant::now() + Duration::from_millis(15))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(handle.stop.load(Ordering::Acquire));
    assert!(handle.join.is_some());
    assert_reserved(dir.path(), &handle);
    release.send(()).unwrap();
    handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
        .await
        .unwrap();
    assert!(handle.join.is_none());
    assert!(handle.retired);
    assert!(!handle.socket_path.exists());
    assert!(
        CidRegistry::open(dir.path())
            .unwrap()
            .lookup(handle.cid)
            .unwrap()
            .is_none()
    );
    handle
        .retire_until(tokio::time::Instant::now())
        .await
        .unwrap();
}

#[tokio::test]
async fn cancelled_retirement_future_keeps_the_same_owner_for_retry() {
    let (release, wait) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |_| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(())
    });
    assert!(
        tokio::time::timeout(
            Duration::from_millis(15),
            handle.retire_until(tokio::time::Instant::now() + Duration::from_secs(2),)
        )
        .await
        .is_err()
    );
    assert_reserved(dir.path(), &handle);
    assert!(handle.join.is_some());
    release.send(()).unwrap();
    handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
        .await
        .unwrap();
}

#[tokio::test]
async fn retirement_does_not_block_the_current_thread_reactor() {
    let runtime = tokio::runtime::Handle::current();
    let (_dir, mut handle) = fixture(move |_| {
        runtime.block_on(async {
            tokio::time::sleep(Duration::from_millis(30)).await;
        });
        Ok(())
    });
    handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
        .await
        .unwrap();
    assert!(handle.retired);
}

#[tokio::test]
async fn failed_or_panicked_loop_never_releases_reservation_on_retry() {
    for panic in [false, true] {
        let (dir, mut handle) = fixture(move |_| {
            assert!(!panic, "synthetic loop panic");
            Err(std::io::Error::other("synthetic failed worker retirement"))
        });
        let error = handle
            .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
            .await
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::Other);
        assert!(
            handle.join.is_none(),
            "the failed join was actually visited"
        );
        assert!(handle.retirement_failure.is_some());
        assert_reserved(dir.path(), &handle);
        assert!(
            handle
                .retire_until(tokio::time::Instant::now())
                .await
                .is_err()
        );
        assert_reserved(dir.path(), &handle);
    }
}

#[test]
fn drop_cancels_without_waiting_or_releasing_a_live_reservation() {
    let (release, wait) = std::sync::mpsc::channel();
    let (done, finished) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |stop| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(stop.load(Ordering::Acquire));
        done.send(()).unwrap();
        Ok(())
    });
    let path = handle.socket_path.clone();
    let registry = CidRegistry::open(dir.path()).unwrap();
    let cid = handle.cid;
    // The fixture retains the exact join independently solely to verify the
    // abandoned-owner branch; production Drop neither joins nor releases.
    let join = handle.join.take().unwrap();
    let before = std::time::Instant::now();
    drop(handle);
    assert!(before.elapsed() < Duration::from_secs(1));
    assert!(path.exists());
    assert!(registry.lookup(cid).unwrap().is_some());
    release.send(()).unwrap();
    finished.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(matches!(
        join.join().unwrap().unwrap(),
        ShimExit::CancelledWithoutFinalization
    ));
    // Even actual joined completion after Drop-only cancellation does not
    // manufacture an explicit reservation-release request.
    assert!(path.exists());
    assert!(registry.lookup(cid).unwrap().is_some());
}

#[tokio::test]
async fn registry_contention_keeps_finalizer_owned_without_blocking_async_deadline() {
    let (release, wait) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |_| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(())
    });
    let reserved = ReservationSnapshot::new(dir.path(), &handle);
    let lock =
        crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(dir.path()).unwrap();
    // Request finalization BEFORE releasing the synthetic relay join.
    handle.request_shutdown();
    release.send(()).unwrap();
    let error = tokio::time::timeout(
        Duration::from_millis(250),
        handle.retire_until(tokio::time::Instant::now() + Duration::from_millis(20)),
    )
    .await
    .expect("registry contention blocked the async reactor")
    .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
    assert!(handle.join.is_some());
    reserved.assert_unchanged(&handle);
    assert!(
        std::os::unix::net::UnixStream::connect(&handle.socket_path).is_err(),
        "listener must close before finalizer waits for registry lock"
    );
    drop(lock);
    handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(2))
        .await
        .unwrap();
    assert!(!handle.socket_path.exists());
    assert!(
        CidRegistry::open(dir.path())
            .unwrap()
            .lookup(handle.cid)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn registry_finalization_failure_keeps_its_cause_and_reservation_sticky() {
    let (release, wait) = std::sync::mpsc::channel();
    let (dir, mut handle) = fixture(move |_| {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        Ok(())
    });
    let reserved = ReservationSnapshot::new(dir.path(), &handle);
    let lock =
        crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(dir.path()).unwrap();
    handle.request_shutdown();
    release.send(()).unwrap();
    let error = handle
        .retire_until(tokio::time::Instant::now() + Duration::from_secs(3))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("timed out acquiring port-registry lock")
    );
    assert!(handle.join.is_none());
    reserved.assert_unchanged(&handle);
    drop(lock);
    let retry = handle
        .retire_until(tokio::time::Instant::now())
        .await
        .unwrap_err();
    assert_eq!(retry.to_string(), error.to_string());
    reserved.assert_unchanged(&handle);
}
