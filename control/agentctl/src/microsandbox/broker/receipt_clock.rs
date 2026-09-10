//! Timestamp complete requests, not the start of a potentially blocking receive.

/// Sample the decision clock only after transport and decoding have succeeded.
/// Keeping reception inside this boundary prevents an idle listener or a slow
/// prelude from making freshness decisions against an earlier wall clock.
pub(super) fn receive_with_clock<T, Stamp>(
    receive: impl FnOnce() -> std::io::Result<T>,
    clock: impl FnOnce() -> Stamp,
) -> std::io::Result<(T, Stamp)> {
    let request = receive()?;
    Ok((request, clock()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn samples_after_receive_even_after_a_long_idle() {
        for elapsed in [0, 1, 300, 301, 600, 86_400] {
            let now = Cell::new(1_700_000_000_u64);
            let receipt_time = now.get() + elapsed;
            let (request, sampled_time) = receive_with_clock(
                || {
                    now.set(receipt_time);
                    Ok("decoded prelude")
                },
                || now.get(),
            )
            .unwrap();
            assert_eq!(request, "decoded prelude");
            assert_eq!(sampled_time, receipt_time, "elapsed: {elapsed}");
        }
    }

    #[test]
    fn samples_exactly_once_after_complete_decoding() {
        let decoded = Cell::new(false);
        let samples = Cell::new(0);
        let (_, timestamp) = receive_with_clock(
            || {
                decoded.set(true);
                Ok(())
            },
            || {
                assert!(decoded.get());
                samples.set(samples.get() + 1);
                "receipt timestamp"
            },
        )
        .unwrap();
        assert_eq!(timestamp, "receipt timestamp");
        assert_eq!(samples.get(), 1);
    }

    #[test]
    fn failed_receive_preserves_error_without_sampling() {
        let samples = Cell::new(0);
        let error = receive_with_clock(
            || Err::<(), _>(std::io::Error::from(std::io::ErrorKind::InvalidData)),
            || samples.set(samples.get() + 1),
        )
        .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(samples.get(), 0);
    }

    #[test]
    fn unavailable_clock_is_not_replaced_with_request_time() {
        let (request_epoch, sampled_time) =
            receive_with_clock(|| Ok(1_700_000_000_u64), || 0_u64).unwrap();
        assert_eq!(request_epoch, 1_700_000_000);
        assert_eq!(sampled_time, 0);
    }
}
