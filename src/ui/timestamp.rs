//! Shared RFC3339 (UTC) "now" formatter used by Records persistence paths
//! across the Quiz and Time Attack 25 UIs.
//!
//! Pulled out of `ui/quiz.rs` so the TA25 finish-flow (#44) can stamp
//! freshly-saved `TimeEntry` rows with the exact same timestamp format
//! that `ui/records.rs` reads back when highlighting the latest entry.
//! Keeping the helper here also avoids dragging in `chrono` for a single
//! line of formatting — the runtime is tiny and dependency-free.

use std::time::{SystemTime, UNIX_EPOCH};

/// Format `SystemTime::now()` as an RFC3339 UTC string
/// (`YYYY-MM-DDTHH:MM:SSZ`). Falls back to the Unix epoch if the system
/// clock is set before 1970 (effectively never, but the math stays
/// well-defined).
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Convert "days since Unix epoch (1970-01-01)" into a proleptic
/// Gregorian (year, month, day) triple. Algorithm from
/// https://howardhinnant.github.io/date_algorithms.html — uses `i64`
/// internally so intermediate subtractions never underflow.
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    let z = days as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u64, m as u64, d as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_rfc3339_matches_format() {
        let ts = now_rfc3339();
        // Expected shape: YYYY-MM-DDTHH:MM:SSZ (20 chars).
        assert_eq!(ts.len(), 20, "unexpected length: {ts}");
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
        assert_eq!(&ts[13..14], ":");
        assert_eq!(&ts[16..17], ":");
        assert_eq!(&ts[19..20], "Z");
        for pos in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
            assert!(ts.as_bytes()[pos].is_ascii_digit());
        }
    }
}
