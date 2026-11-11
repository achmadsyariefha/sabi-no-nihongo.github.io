//! Month grids for the Calendar page.

use std::collections::BTreeMap;

use crate::DateKey;
use crate::date::days_in_month;
use crate::model::DailyEntry;

/// A calendar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month {
    year: i32,
    month: u8,
}

pub const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];

impl Month {
    pub fn of(day: DateKey) -> Self {
        Self { year: day.year(), month: day.month() }
    }

    pub fn year(self) -> i32 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn first_day(self) -> DateKey {
        DateKey::new(self.year, self.month, 1).expect("day 1 always exists")
    }

    pub fn day_count(self) -> u8 {
        days_in_month(self.year, self.month)
    }

    pub fn contains(self, day: DateKey) -> bool {
        Month::of(day) == self
    }

    pub fn next(self) -> Self {
        if self.month == 12 { Self { year: self.year + 1, month: 1 } } else { Self { month: self.month + 1, ..self } }
    }

    pub fn prev(self) -> Self {
        if self.month == 1 { Self { year: self.year - 1, month: 12 } } else { Self { month: self.month - 1, ..self } }
    }

    /// e.g. `November 2026 · 11月`.
    pub fn label(self) -> String {
        format!("{} {} · {}月", MONTH_NAMES[self.month as usize - 1], self.year, self.month)
    }

    /// The grid shown on the page: Sunday-first weeks, `None` for the blank
    /// slots before day 1 and after the last day (so rows are always full).
    pub fn cells(self) -> Vec<Option<DateKey>> {
        let first = self.first_day();
        let mut cells: Vec<Option<DateKey>> = vec![None; first.weekday() as usize];
        let mut d = first;
        for _ in 0..self.day_count() {
            cells.push(Some(d));
            d = d.next();
        }
        while !cells.len().is_multiple_of(7) {
            cells.push(None);
        }
        cells
    }
}

/// What a day cell shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaySummary {
    /// The first word logged that day.
    pub first_japanese: String,
    pub first_translation: String,
    pub count: usize,
    pub any_favorite: bool,
}

/// One summary per logged day.
pub fn summarize(entries: &[DailyEntry]) -> BTreeMap<DateKey, DaySummary> {
    let mut sorted: Vec<&DailyEntry> = entries.iter().collect();
    sorted.sort_by_key(|e| e.created_at);
    let mut out: BTreeMap<DateKey, DaySummary> = BTreeMap::new();
    for e in sorted {
        out.entry(e.date)
            .and_modify(|s| {
                s.count += 1;
                s.any_favorite |= e.is_favorite;
            })
            .or_insert_with(|| DaySummary {
                first_japanese: e.japanese.clone(),
                first_translation: e.translation.clone(),
                count: 1,
                any_favorite: e.is_favorite,
            });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> DateKey {
        s.parse().unwrap()
    }

    #[test]
    fn next_and_prev_wrap_years() {
        let dec = Month::of(d("2026-12-05"));
        assert_eq!(dec.next(), Month::of(d("2027-01-01")));
        assert_eq!(Month::of(d("2027-01-31")).prev(), dec);
        assert_eq!(dec.label(), "December 2026 · 12月");
    }

    #[test]
    fn cells_start_on_the_right_weekday() {
        // 2026-11-01 is a Sunday: no leading blanks, 30 days, 5 rows.
        let nov = Month::of(d("2026-11-11")).cells();
        assert_eq!(nov[0], Some(d("2026-11-01")));
        assert_eq!(nov.len(), 35);
        assert_eq!(nov.iter().flatten().count(), 30);

        // 2026-09-01 is a Tuesday: two blanks first.
        let sep = Month::of(d("2026-09-01")).cells();
        assert_eq!(&sep[..3], &[None, None, Some(d("2026-09-01"))]);
        assert_eq!(sep.len() % 7, 0);
    }

    #[test]
    fn summarizes_days_by_first_word() {
        use crate::model::EntryDraft;
        let make = |jp: &str, date: &str, created: i64, fav: bool| {
            let draft =
                EntryDraft { japanese: jp.into(), translation: "t".into(), is_favorite: fav, ..Default::default() };
            draft.build(None, || jp.into(), d(date), created).unwrap()
        };
        let list = [make("後", "2026-11-11", 20, true), make("先", "2026-11-11", 10, false), make("昨", "2026-11-10", 5, false)];
        let days = summarize(&list);
        let today = &days[&d("2026-11-11")];
        assert_eq!(today.first_japanese, "先");
        assert_eq!(today.count, 2);
        assert!(today.any_favorite);
        assert!(!days[&d("2026-11-10")].any_favorite);
    }

    #[test]
    fn leap_february() {
        let feb = Month::of(d("2024-02-10"));
        assert_eq!(feb.day_count(), 29);
        assert!(feb.contains(d("2024-02-29")));
        assert!(!feb.contains(d("2024-03-01")));
    }
}
