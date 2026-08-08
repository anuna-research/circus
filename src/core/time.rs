//! RFC 3339 timestamp formatting, in UTC.
//!
//! `SPEC-001-circus-agent-harness#CON-007` requires every timestamp to be
//! RFC 3339 with a `Z` offset. The standard library measures time but does not
//! format calendar dates, so the conversion lives here.
//!
//! Ladder note: rung 4 was considered and passed over. A date-formatting
//! dependency would be carried for one function, and the exact algorithm below
//! (Howard Hinnant's `civil_from_days`) is correct for every representable
//! date rather than only the common ones. Being pure, it is also directly
//! testable, which a formatting crate's internals are not.

use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch. A newtype so a duration cannot be passed
/// where an instant is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnixSeconds(pub i64);

impl UnixSeconds {
    /// Read the wall clock. This is the one impure function in the core, and
    /// it is confined to this constructor so every other function here takes
    /// the instant as an argument and stays deterministic.
    pub fn now() -> Self {
        let d = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is at or after 1970");
        Self(d.as_secs() as i64)
    }
}

/// Format an instant as `YYYY-MM-DDThh:mm:ssZ`.
pub fn format_rfc3339(t: UnixSeconds) -> String {
    let secs = t.0;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// The current instant, formatted.
pub fn now_rfc3339() -> String {
    format_rfc3339(UnixSeconds::now())
}

/// Days since 1970-01-01 to a proleptic Gregorian calendar date.
///
/// Howard Hinnant, *`chrono`-Compatible Low-Level Date Algorithms*. Exact for
/// every value the return type can hold.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (y + i64::from(m <= 2), m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_instants() {
        // Values cross-checked against `date -u -r <secs>`.
        let cases = [
            (0i64, "1970-01-01T00:00:00Z"),
            (1, "1970-01-01T00:00:01Z"),
            (86_399, "1970-01-01T23:59:59Z"),
            (86_400, "1970-01-02T00:00:00Z"),
            (951_782_400, "2000-02-29T00:00:00Z"), // leap day in a century leap year
            (1_709_164_800, "2024-02-29T00:00:00Z"), // leap day
            (1_754_611_200, "2025-08-08T00:00:00Z"),
            (2_147_483_647, "2038-01-19T03:14:07Z"), // past the 32-bit cliff
            (4_102_444_800, "2100-01-01T00:00:00Z"), // 2100 is not a leap year
        ];
        for (secs, want) in cases {
            assert_eq!(format_rfc3339(UnixSeconds(secs)), want, "for {secs}");
        }
    }

    #[test]
    fn handles_dates_before_the_gregorian_era_pivot() {
        // Exercises the `z - 146_096` branch of civil_from_days, which only
        // runs when the shifted day count goes negative — that is, before
        // 0000-03-01. Vectors computed from the inverse algorithm
        // (days_from_civil), not from this function.
        let cases = [
            (-62_162_035_200i64, "0000-03-01T00:00:00Z"), // the pivot itself
            (-62_162_121_600, "0000-02-29T00:00:00Z"),    // one day before it
            (-62_167_219_200, "0000-01-01T00:00:00Z"),
        ];
        for (secs, want) in cases {
            assert_eq!(format_rfc3339(UnixSeconds(secs)), want, "for {secs}");
        }
    }

    #[test]
    fn handles_instants_before_the_epoch() {
        assert_eq!(
            format_rfc3339(UnixSeconds(-1)),
            "1969-12-31T23:59:59Z",
            "negative seconds must floor, not truncate toward zero"
        );
    }

    #[test]
    fn every_day_across_four_centuries_is_monotonic_and_well_formed() {
        // A property test without a framework: walk 400 years of days and
        // assert the formatted strings are strictly increasing. Any arithmetic
        // slip in civil_from_days breaks ordering somewhere in this range.
        let mut prev = String::new();
        let mut day = -25_567i64; // 1900-01-01
        while day < 120_000 {
            let s = format_rfc3339(UnixSeconds(day * 86_400));
            assert_eq!(s.len(), 20, "malformed: {s}");
            assert!(s > prev, "not monotonic at day {day}: {prev} then {s}");
            prev = s;
            day += 1;
        }
    }

    #[test]
    fn now_is_formatted_and_plausible() {
        let s = now_rfc3339();
        assert_eq!(s.len(), 20);
        assert!(s.ends_with('Z'));
        assert!(
            s.as_str() > "2020-01-01T00:00:00Z",
            "clock looks wrong: {s}"
        );
    }
}
