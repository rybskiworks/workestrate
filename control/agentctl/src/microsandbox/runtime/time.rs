/// Current time as an RFC3339 UTC string. Best-effort: falls back to the
/// Unix timestamp when SystemTime fails (should not happen in practice).
pub(super) fn current_rfc3339_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as a stable RFC3339-ish UTC timestamp using a tiny ad-hoc
    // formatter (no chrono dep). The format is `YYYY-MM-DDTHH:MM:SSZ`. This
    // is good enough for `ps` display; callers needing finer precision can
    // post-process. The date math is the standard days-from-civil algorithm
    // (Howard Hinnant, http://howardhinnant.github.io/date_algorithms.html).
    let days = (now / 86_400) as i64;
    let secs = (now % 86_400) as u32;
    let (y, m, d) = days_to_ymd(days);
    let hh = secs / 3600;
    let mm = (secs % 3600) / 60;
    let ss = secs % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Convert days-since-1970-01-01 to (year, month, day). Pure.
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{current_rfc3339_utc, days_to_ymd};

    #[test]
    fn current_rfc3339_utc_ends_with_z_and_has_expected_length() {
        // Sanity: format is YYYY-MM-DDTHH:MM:SSZ = 20 chars.
        let ts = current_rfc3339_utc();
        assert!(
            ts.len() == 20 && ts.ends_with('Z'),
            "unexpected rfc3339 shape: {ts}"
        );
    }

    #[test]
    fn days_to_ymd_known_epoch() {
        // 1970-01-01 is day 0.
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 1970-01-02 is day 1.
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
        // 1971-01-01 is day 365 (1970 was NOT a leap year).
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
        // 2026-01-01: count of days from 1970-01-01.
        // (20454 days; cross-checked against `date -d 2026-01-01 +%s` /86400.)
        assert_eq!(days_to_ymd(20_454), (2026, 1, 1));
    }
}
