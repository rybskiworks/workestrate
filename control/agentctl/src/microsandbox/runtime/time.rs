/// Current time as an RFC3339 UTC string. Best-effort: falls back to the
/// Unix timestamp when SystemTime fails (should not happen in practice).
///
/// `pub(crate)`: reused by `images::state` (spec 21 §8 `built_at`/`loaded_at`)
/// so the crate keeps ONE no-chrono RFC3339 formatter.
pub(crate) fn current_rfc3339_utc() -> String {
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

/// Parse the fixed-shape RFC3339 UTC timestamp emitted by
/// [`current_rfc3339_utc`] (`YYYY-MM-DDTHH:MM:SSZ`) into Unix epoch seconds.
/// Returns None for any other shape (legacy/empty/corrupt `created_at`).
pub(crate) fn parse_rfc3339_utc(s: &str) -> Option<u64> {
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    let bytes = s.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return None;
    }
    let digit = |b: u8| b.is_ascii_digit();
    for i in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !digit(bytes[i]) {
            return None;
        }
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let minute: u32 = s[14..16].parse().ok()?;
    let second: u32 = s[17..19].parse().ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    // days-from-civil (inverse of days_to_ymd), Howard Hinnant.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = ((153 * mp + 2) / 5 + day - 1) as i64;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days as u64 * 86_400 + hour as u64 * 3_600 + minute as u64 * 60 + second as u64)
}

/// Seconds between now and a `created_at` RFC3339 UTC string, when it parses.
/// `None` for unparseable/empty timestamps (legacy records).
pub(crate) fn record_age_secs(created_at: &str) -> Option<u64> {
    let created = parse_rfc3339_utc(created_at)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(now.saturating_sub(created))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{current_rfc3339_utc, days_to_ymd, parse_rfc3339_utc};

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
    fn parse_rfc3339_utc_round_trips_current() {
        let ts = current_rfc3339_utc();
        assert!(
            parse_rfc3339_utc(&ts).is_some(),
            "current formatter output must parse back: {ts}"
        );
    }

    #[test]
    fn parse_rfc3339_utc_known_epoch() {
        assert_eq!(parse_rfc3339_utc("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_rfc3339_utc("2026-01-01T00:00:00Z"),
            Some(1_767_225_600)
        );
    }

    #[test]
    fn parse_rfc3339_utc_rejects_bad_shapes() {
        for bad in [
            "",
            "1970-01-01T00:00:00",   // missing Z
            "1970-01-01 00:00:00Z",  // space not T
            "1970-13-01T00:00:00Z",  // bad month
            "1970-01-32T00:00:00Z",  // bad day
            "1970-01-01T24:00:00Z",  // bad hour
            "x970-01-01T00:00:00Z",  // bad digit
            "1970-01-01T00:00:00ZZ", // extra
        ] {
            assert_eq!(
                parse_rfc3339_utc(bad),
                None,
                "must reject malformed timestamp: {bad}"
            );
        }
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
