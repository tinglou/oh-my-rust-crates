//! Registration-date day-count encoding (`no_std`, `alloc`-free except for formatting)
//!
//! Registration dates are stored as a `u16` count of days since the epoch
//! **1998-01-01** (day `0`). The value `u16::MAX` is reserved as a sentinel for
//! "no date"; real dates therefore span 1998-01-01 ..= 2177-06-06 (65534 days).
//!
//! The Gregorian conversions are implemented locally (no external `time`/`chrono`
//! dependency) and are usable in `const` contexts so generated code can embed
//! pre-computed values.

use alloc::string::String;

/// Epoch year: `1998-01-01` maps to day `0`.
pub const EPOCH_YEAR: u32 = 1998;

/// Sentinel day value meaning "no registration date".
pub const SENTINEL: u16 = u16::MAX;

/// Maximum representable day value (one less than [`SENTINEL`]).
pub const MAX_DAY: u16 = SENTINEL - 1;

/// Returns `true` if `year` is a leap year in the proleptic Gregorian calendar.
#[allow(clippy::manual_is_multiple_of)] // `is_multiple_of` is not usable in `const fn`
const fn is_leap(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1..=12) of `year`.
const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// Days from `0000-03-01` to `y-m-d`, counting (year=0, month=3) as day 0.
///
/// This is the standard "days from civil" formulation; it is only used
/// internally by [`ymd_to_days`] / [`days_to_ymd`].
const fn days_from_civil(y: u32, m: u32, d: u32) -> u32 {
    // Shift the year so that March is the first month (puts the leap day at the
    // end of the shifted year, simplifying the leap adjustment).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if m > 2 { m - 3 } else { m + 9 }; // [0, 11], March = 0
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe
}

/// Days since `1998-01-01` for the given (proleptic Gregorian) date.
///
/// `month` is `1..=12`, `day` is `1..=31`. Returns `None` when the date is
/// invalid or falls outside the encodable range `1998-01-01 ..= 2177-06-06`.
pub const fn ymd_to_days(year: u32, month: u32, day: u32) -> Option<u16> {
    if month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return None;
    }
    if year < EPOCH_YEAR {
        return None;
    }
    let days = days_from_civil(year, month, day) - days_from_civil(EPOCH_YEAR, 1, 1);
    if days > MAX_DAY as u32 {
        return None;
    }
    Some(days as u16)
}

/// Inverse of [`ymd_to_days`]: converts a day count back to `(year, month, day)`.
///
/// The caller must pass a value `<= MAX_DAY` (the sentinel has no date meaning).
pub const fn days_to_ymd(days: u16) -> (u32, u32, u32) {
    let z = days as u32 + days_from_civil(EPOCH_YEAR, 1, 1);
    let era = z / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Parses a strict `YYYYMMDD` byte slice into a day count since 1998-01-01.
///
/// Only exactly 8 ASCII digits that form a valid, in-range calendar date are
/// accepted; anything else returns `None`.
pub const fn parse_yyyymmdd(bytes: &[u8]) -> Option<u16> {
    if bytes.len() != 8 {
        return None;
    }
    let mut i = 0;
    while i < 8 {
        if !bytes[i].is_ascii_digit() {
            return None;
        }
        i += 1;
    }
    let year =
        digit(bytes[0]) * 1000 + digit(bytes[1]) * 100 + digit(bytes[2]) * 10 + digit(bytes[3]);
    let month = digit(bytes[4]) * 10 + digit(bytes[5]);
    let day = digit(bytes[6]) * 10 + digit(bytes[7]);
    ymd_to_days(year, month, day)
}

const fn digit(b: u8) -> u32 {
    (b - b'0') as u32
}

/// Formats a day count as `"YYYY-MM-DD"`. The [`SENTINEL`] formats as `""`.
pub fn format_date(days: u16) -> String {
    if days == SENTINEL {
        return String::new();
    }
    let (y, m, d) = days_to_ymd(days);
    alloc::format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_zero() {
        assert_eq!(ymd_to_days(1998, 1, 1), Some(0));
        assert_eq!(days_to_ymd(0), (1998, 1, 1));
    }

    #[test]
    fn leap_day() {
        // 2000 is a leap year (divisible by 400)
        let feb29 = ymd_to_days(2000, 2, 29).unwrap();
        assert_eq!(days_to_ymd(feb29), (2000, 2, 29));
        // 1900 is not a leap year, but it is before the epoch -> None
        assert_eq!(ymd_to_days(1900, 2, 29), None);
        // 2023 is not a leap year
        assert_eq!(ymd_to_days(2023, 2, 29), None);
    }

    #[test]
    fn round_trip_range() {
        for days in (0..=MAX_DAY).step_by(37) {
            let (y, m, d) = days_to_ymd(days);
            assert_eq!(
                ymd_to_days(y, m, d),
                Some(days),
                "round-trip failed at {days}"
            );
        }
        // Bounds
        assert_eq!(
            ymd_to_days(
                days_to_ymd(MAX_DAY).0,
                days_to_ymd(MAX_DAY).1,
                days_to_ymd(MAX_DAY).2
            ),
            Some(MAX_DAY)
        );
        assert_eq!(ymd_to_days(2177, 6, 7), None); // one past MAX_DAY
    }

    #[test]
    fn parse_and_format() {
        assert_eq!(parse_yyyymmdd(b"19980101"), Some(0));
        assert_eq!(format_date(0), "1998-01-01");

        let d = parse_yyyymmdd(b"20230719").unwrap();
        assert_eq!(format_date(d), "2023-07-19");

        // Rejections
        assert_eq!(parse_yyyymmdd(b"2023071"), None); // 7 digits
        assert_eq!(parse_yyyymmdd(b"202307199"), None); // 9 digits
        assert_eq!(parse_yyyymmdd(b"2023071X"), None); // non-digit
        assert_eq!(parse_yyyymmdd(b"18991231"), None); // before epoch
        assert_eq!(parse_yyyymmdd(b"---"), None);
    }

    #[test]
    fn sentinel_formats_empty() {
        assert_eq!(format_date(SENTINEL), "");
    }
}
