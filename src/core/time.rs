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

/// Read an instant back from `YYYY-MM-DDThh:mm:ssZ`.
///
/// The inverse of [`format_rfc3339`], and deliberately no more general than
/// that: Circus only ever parses timestamps it wrote itself, so accepting
/// offsets or fractional seconds would be surface with no caller.
pub fn parse_rfc3339(s: &str) -> Result<UnixSeconds, super::RecognitionError> {
    const P: &str = "rfc3339";
    let err = |d: &str| super::RecognitionError::new(P, d.to_owned());
    let b = s.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return Err(err("not YYYY-MM-DDThh:mm:ssZ"));
    }
    let num = |a: usize, z: usize| -> Result<i64, super::RecognitionError> {
        s[a..z].parse().map_err(|_| err("non-numeric field"))
    };
    let (y, mo, d) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    let (h, mi, sec) = (num(11, 13)?, num(14, 16)?, num(17, 19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || sec > 60 {
        return Err(err("field out of range"));
    }
    Ok(UnixSeconds(
        days_from_civil(y, mo as u32, d as u32) * 86_400 + h * 3600 + mi * 60 + sec,
    ))
}

/// A proleptic Gregorian calendar date to days since 1970-01-01.
///
/// Hinnant's `days_from_civil`, the exact inverse of [`civil_from_days`].
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64; // [0, 11]
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
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
    fn parsing_inverts_formatting_across_four_centuries() {
        // The two algorithms are each other's inverse or they are both wrong;
        // walking the range is the cheapest way to hold them to it.
        let mut day = -25_567i64; // 1900-01-01
        while day < 120_000 {
            let t = UnixSeconds(day * 86_400 + 3_661);
            let rendered = format_rfc3339(t);
            assert_eq!(parse_rfc3339(&rendered).unwrap(), t, "at {rendered}");
            day += 1;
        }
    }

    #[test]
    fn parses_the_instants_it_formats() {
        for secs in [
            0i64,
            1,
            86_399,
            951_782_400,
            1_754_611_200,
            2_147_483_647,
            -1,
        ] {
            let t = UnixSeconds(secs);
            assert_eq!(parse_rfc3339(&format_rfc3339(t)).unwrap(), t);
        }
    }

    #[test]
    fn refuses_anything_it_did_not_write() {
        for bad in [
            "",
            "2026-08-09",
            "2026-08-09T00:00:00",
            "2026-08-09T00:00:00+01:00",
            "2026-08-09T00:00:00.5Z",
            "2026-13-01T00:00:00Z",
            "2026-08-32T00:00:00Z",
            "2026-08-09T24:00:00Z",
            "2026-08-09T00:60:00Z",
            "20x6-08-09T00:00:00Z",
            "2026/08/09T00:00:00Z",
        ] {
            assert!(parse_rfc3339(bad).is_err(), "accepted {bad:?}");
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
