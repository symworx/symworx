// Copyright (c) 2026 Nathaniel T. Berry
// Licensed under the Apache License, Version 2.0.

//! Civil time and explicit timezone offsets (no `chrono`).
//!
//! Callers pick a [`TimeZone`]: fixed `EST` / `EDT`, DST-aware `UsEastern`,
//! `Utc`, or a pasted hour offset.

/// Naive civil wall time (no zone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivilTime {
    /// Year (e.g. 2017).
    pub year: i32,
    /// Month 1–12.
    pub month: u8,
    /// Day of month 1–31.
    pub day: u8,
    /// Hour 0–23.
    pub hour: u8,
    /// Minute 0–59.
    pub minute: u8,
    /// Second 0–60 (leap second stored, not special-cased).
    pub second: u8,
}

impl CivilTime {
    /// Construct after range checks.
    pub fn new(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > 31 || hour > 23 || minute > 59 || second > 60 {
            return None;
        }
        Some(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        })
    }
}

/// Named zone or an explicit UTC offset the caller pastes in.
///
/// `Est` / `Edt` are **fixed** (no DST). `UsEastern` applies US DST rules in
/// effect since 2007 (2nd Sunday of March 02:00 → 1st Sunday of November 02:00).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeZone {
    /// UTC, offset 0.
    Utc,
    /// US Eastern Standard, UTC−5, no DST.
    Est,
    /// US Eastern Daylight, UTC−4, no DST.
    Edt,
    /// America/New_York: EST or EDT from the civil date (US rules, 2007+).
    UsEastern,
    /// Explicit offset in **hours** east of UTC (e.g. `-5` for EST).
    FixedHours(i8),
}

impl TimeZone {
    /// Parse `"UTC"`, `"EST"`, `"EDT"`, `"US/Eastern"` / `"America/New_York"`,
    /// or `"UTC-5"` / `"-05"` / `"+1"`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let t = s.trim();
        match t.to_ascii_uppercase().as_str() {
            "UTC" | "Z" | "GMT" => return Ok(Self::Utc),
            "EST" => return Ok(Self::Est),
            "EDT" => return Ok(Self::Edt),
            "US/EASTERN" | "AMERICA/NEW_YORK" | "EASTERN" | "US-EASTERN" => return Ok(Self::UsEastern),
            _ => {}
        }
        let lower = t.to_ascii_lowercase();
        if lower == "us/eastern" || lower == "america/new_york" {
            return Ok(Self::UsEastern);
        }
        if let Some(hours) = parse_fixed_hours(t) {
            return Ok(Self::FixedHours(hours));
        }
        Err(format!("unknown time zone {s:?}"))
    }

    /// Offset from UTC in seconds (east positive) at this **local** civil time.
    pub fn offset_s(self, local: CivilTime) -> i32 {
        match self {
            Self::Utc => 0,
            Self::Est => -5 * 3600,
            Self::Edt => -4 * 3600,
            Self::FixedHours(h) => i32::from(h) * 3600,
            Self::UsEastern => {
                if is_us_eastern_dst(local) {
                    -4 * 3600
                } else {
                    -5 * 3600
                }
            }
        }
    }

    /// Offset in hours at this local civil time.
    pub fn offset_hours(self, local: CivilTime) -> i8 {
        (self.offset_s(local) / 3600) as i8
    }

    /// Convert local civil time in this zone to UNIX seconds (UTC).
    pub fn local_to_unix(self, local: CivilTime) -> Option<i64> {
        let as_utc = civil_to_unix(local)?;
        Some(as_utc - i64::from(self.offset_s(local)))
    }

    /// Convert UNIX seconds (UTC) to local civil time in this zone.
    pub fn unix_to_local(self, unix_s: i64) -> Option<CivilTime> {
        let off = match self {
            Self::UsEastern => {
                let (start, end) = us_eastern_dst_unix_bounds(unix_to_civil(unix_s)?.year)?;
                if unix_s >= start && unix_s < end {
                    -4 * 3600
                } else {
                    -5 * 3600
                }
            }
            other => other.offset_s(CivilTime {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            }),
        };
        unix_to_civil(unix_s + i64::from(off))
    }
}

/// US Eastern DST (Energy Policy Act, 2007–): 2nd Sunday of March 02:00 local
/// through 1st Sunday of November 02:00 local. The 01:00–02:00 fall overlap is
/// treated as still DST (first occurrence).
pub fn is_us_eastern_dst(local: CivilTime) -> bool {
    let y = local.year;
    let m = local.month;
    let d = local.day;
    let h = local.hour;
    if m < 3 || m > 11 {
        return false;
    }
    if m > 3 && m < 11 {
        return true;
    }
    if m == 3 {
        let start = nth_weekday(y, 3, 0, 2);
        return d > start || (d == start && h >= 2);
    }
    let end = nth_weekday(y, 11, 0, 1);
    d < end || (d == end && h < 2)
}

fn parse_fixed_hours(s: &str) -> Option<i8> {
    let t = s.trim().trim_start_matches("UTC").trim_start_matches("utc");
    let t = t.trim();
    if t.is_empty() {
        return None;
    }
    let t = t.strip_prefix('+').unwrap_or(t);
    if let Some(hh) = t.split(':').next() {
        return hh.parse::<i8>().ok().filter(|h| (-12..=14).contains(h));
    }
    None
}

/// Howard Hinnant: days since 1970-01-01.
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y as i64;
    let m = m as i64;
    let d = d as i64;
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

fn civil_to_unix(c: CivilTime) -> Option<i64> {
    CivilTime::new(c.year, c.month, c.day, c.hour, c.minute, c.second)?;
    let days = days_from_civil(c.year, u32::from(c.month), u32::from(c.day));
    Some(days * 86400 + i64::from(c.hour) * 3600 + i64::from(c.minute) * 60 + i64::from(c.second))
}

fn unix_to_civil(unix_s: i64) -> Option<CivilTime> {
    let days = unix_s.div_euclid(86400);
    let rem = unix_s.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    let hour = (rem / 3600) as u8;
    let minute = ((rem % 3600) / 60) as u8;
    let second = (rem % 60) as u8;
    CivilTime::new(y, m as u8, d as u8, hour, minute, second)
}

fn us_eastern_dst_unix_bounds(year: i32) -> Option<(i64, i64)> {
    let start_day = nth_weekday(year, 3, 0, 2);
    let end_day = nth_weekday(year, 11, 0, 1);
    let start = TimeZone::Est.local_to_unix(CivilTime {
        year,
        month: 3,
        day: start_day,
        hour: 2,
        minute: 0,
        second: 0,
    })?;
    let end = TimeZone::Edt.local_to_unix(CivilTime {
        year,
        month: 11,
        day: end_day,
        hour: 2,
        minute: 0,
        second: 0,
    })?;
    Some((start, end))
}

/// `weekday`: 0 = Sunday. `n`: 1-based occurrence in the month.
fn nth_weekday(year: i32, month: u8, weekday: u8, n: u8) -> u8 {
    let days = days_from_civil(year, u32::from(month), 1);
    // 1970-01-01 was Thursday. Sunday = 0 → (days + 4) rem 7.
    let wd = (days + 4).rem_euclid(7) as u8;
    let first = if wd <= weekday {
        1 + weekday - wd
    } else {
        1 + 7 - (wd - weekday)
    };
    first + 7 * (n - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ct(y: i32, mo: u8, d: u8, h: u8, mi: u8, s: u8) -> CivilTime {
        CivilTime::new(y, mo, d, h, mi, s).unwrap()
    }

    #[test]
    fn parse_names() {
        assert_eq!(TimeZone::parse("EST").unwrap(), TimeZone::Est);
        assert_eq!(TimeZone::parse("EDT").unwrap(), TimeZone::Edt);
        assert_eq!(TimeZone::parse("America/New_York").unwrap(), TimeZone::UsEastern);
        assert_eq!(TimeZone::parse("UTC-5").unwrap(), TimeZone::FixedHours(-5));
        assert_eq!(TimeZone::parse("+1").unwrap(), TimeZone::FixedHours(1));
    }

    #[test]
    fn august_2017_is_edt() {
        let local = ct(2017, 8, 14, 8, 20, 0);
        assert!(is_us_eastern_dst(local));
        assert_eq!(TimeZone::UsEastern.offset_hours(local), -4);
        assert_eq!(TimeZone::Est.offset_hours(local), -5);
        assert_eq!(TimeZone::Edt.offset_hours(local), -4);
    }

    #[test]
    fn january_is_est() {
        let local = ct(2018, 1, 15, 6, 0, 0);
        assert!(!is_us_eastern_dst(local));
        assert_eq!(TimeZone::UsEastern.offset_hours(local), -5);
    }

    #[test]
    fn dst_start_2017_second_sunday_march() {
        assert!(!is_us_eastern_dst(ct(2017, 3, 12, 1, 59, 0)));
        assert!(is_us_eastern_dst(ct(2017, 3, 12, 3, 0, 0)));
    }

    #[test]
    fn dst_end_2017_first_sunday_november() {
        assert!(is_us_eastern_dst(ct(2017, 11, 5, 1, 30, 0)));
        assert!(!is_us_eastern_dst(ct(2017, 11, 5, 2, 0, 0)));
    }

    #[test]
    fn local_to_unix_edt() {
        // 2017-08-14 08:20 EDT = 12:20 UTC
        let unix = TimeZone::UsEastern.local_to_unix(ct(2017, 8, 14, 8, 20, 0)).unwrap();
        let utc = TimeZone::Utc.local_to_unix(ct(2017, 8, 14, 12, 20, 0)).unwrap();
        assert_eq!(unix, utc);
        let back = TimeZone::UsEastern.unix_to_local(unix).unwrap();
        assert_eq!(back, ct(2017, 8, 14, 8, 20, 0));
    }

    #[test]
    fn unix_to_local_after_november_fallback() {
        // 2017-11-05 07:00 UTC = 02:00 EST
        let unix = TimeZone::Utc.local_to_unix(ct(2017, 11, 5, 7, 0, 0)).unwrap();
        let local = TimeZone::UsEastern.unix_to_local(unix).unwrap();
        assert_eq!(local, ct(2017, 11, 5, 2, 0, 0));
        assert_eq!(TimeZone::UsEastern.offset_hours(local), -5);
    }

    #[test]
    fn civil_unix_roundtrip_epoch() {
        let c = ct(1970, 1, 1, 0, 0, 0);
        assert_eq!(civil_to_unix(c).unwrap(), 0);
        assert_eq!(unix_to_civil(0).unwrap(), c);
    }
}
