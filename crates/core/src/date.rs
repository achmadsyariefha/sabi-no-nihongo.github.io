//! Calendar dates as `YYYY-MM-DD` keys, the format used in storage and backups.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A calendar day with no time or timezone. Serializes as `"YYYY-MM-DD"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateKey {
    year: i32,
    month: u8,
    day: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidDate(pub String);

impl fmt::Display for InvalidDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid date {:?}, expected YYYY-MM-DD", self.0)
    }
}

impl std::error::Error for InvalidDate {}

const WEEKDAYS_JA: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];

impl DateKey {
    /// Builds a date, returning `None` if it does not exist (e.g. Feb 30).
    pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return None;
        }
        if day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }

    /// Day of week, 0 = Sunday … 6 = Saturday.
    pub fn weekday(self) -> u8 {
        // Sakamoto's method.
        const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
        let mut y = self.year;
        if self.month < 3 {
            y -= 1;
        }
        let w = (y + y / 4 - y / 100 + y / 400 + T[self.month as usize - 1] + self.day as i32) % 7;
        w as u8
    }

    /// The following day.
    pub fn next(self) -> Self {
        if self.day < days_in_month(self.year, self.month) {
            Self { day: self.day + 1, ..self }
        } else if self.month < 12 {
            Self { month: self.month + 1, day: 1, ..self }
        } else {
            Self { year: self.year + 1, month: 1, day: 1 }
        }
    }

    /// The previous day.
    pub fn prev(self) -> Self {
        if self.day > 1 {
            Self { day: self.day - 1, ..self }
        } else if self.month > 1 {
            let month = self.month - 1;
            Self { month, day: days_in_month(self.year, month), ..self }
        } else {
            Self { year: self.year - 1, month: 12, day: 31 }
        }
    }

    /// Japanese style, e.g. `2026年11月11日（水）`.
    pub fn format_ja(self) -> String {
        format!(
            "{}年{}月{}日（{}）",
            self.year,
            self.month,
            self.day,
            WEEKDAYS_JA[self.weekday() as usize]
        )
    }
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl fmt::Display for DateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for DateKey {
    type Err = InvalidDate;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || InvalidDate(s.to_owned());
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(err());
        }
        let digits = |r: std::ops::Range<usize>| -> Result<u32, InvalidDate> {
            let part = &s[r];
            if part.bytes().all(|c| c.is_ascii_digit()) {
                part.parse().map_err(|_| err())
            } else {
                Err(err())
            }
        };
        let year = digits(0..4)? as i32;
        let month = digits(5..7)? as u8;
        let day = digits(8..10)? as u8;
        DateKey::new(year, month, day).ok_or_else(err)
    }
}

impl Serialize for DateKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for DateKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> DateKey {
        s.parse().unwrap()
    }

    #[test]
    fn parses_and_displays() {
        assert_eq!(d("2026-11-11").to_string(), "2026-11-11");
        assert_eq!(d("0999-01-05").to_string(), "0999-01-05");
    }

    #[test]
    fn rejects_bad_input() {
        for bad in ["", "2026-1-11", "2026/11/11", "2026-13-01", "2026-02-30", "2025-02-29", "20a6-11-11", "2026-11-11T00"] {
            assert!(bad.parse::<DateKey>().is_err(), "{bad} should be rejected");
        }
        assert!("2024-02-29".parse::<DateKey>().is_ok());
    }

    #[test]
    fn weekday_and_japanese_format() {
        assert_eq!(d("2026-11-11").format_ja(), "2026年11月11日（水）");
        assert_eq!(d("2026-09-01").format_ja(), "2026年9月1日（火）");
        assert_eq!(d("2000-01-01").weekday(), 6);
    }

    #[test]
    fn next_and_prev_cross_boundaries() {
        assert_eq!(d("2026-12-31").next(), d("2027-01-01"));
        assert_eq!(d("2024-02-28").next(), d("2024-02-29"));
        assert_eq!(d("2025-02-28").next(), d("2025-03-01"));
        assert_eq!(d("2026-03-01").prev(), d("2026-02-28"));
        assert_eq!(d("2026-01-01").prev(), d("2025-12-31"));
    }

    #[test]
    fn ordering_matches_string_order() {
        assert!(d("2026-09-30") < d("2026-10-01"));
    }

    #[test]
    fn serde_round_trip() {
        let json = serde_json::to_string(&d("2026-08-30")).unwrap();
        assert_eq!(json, "\"2026-08-30\"");
        let back: DateKey = serde_json::from_str(&json).unwrap();
        assert_eq!(back, d("2026-08-30"));
        assert!(serde_json::from_str::<DateKey>("\"2026-02-31\"").is_err());
    }
}
