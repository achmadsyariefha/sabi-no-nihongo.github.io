//! Per-day activity counts and the heatmap layout built from them.

use std::collections::BTreeMap;

use crate::DateKey;
use crate::model::DailyEntry;

/// Count of something (words added, cards reviewed) per day.
pub type DayCounts = BTreeMap<DateKey, u32>;

/// Words logged on each day.
pub fn words_per_day(entries: &[DailyEntry]) -> DayCounts {
    let mut counts = DayCounts::new();
    for e in entries {
        *counts.entry(e.date).or_default() += 1;
    }
    counts
}

/// Builds a review log from stored `(date, count)` pairs, dropping bad dates
/// and zero counts. Returns the log and how many pairs were dropped.
pub fn review_log_from_pairs<'a>(pairs: impl IntoIterator<Item = (&'a str, Option<u64>)>) -> (DayCounts, usize) {
    let mut log = DayCounts::new();
    let mut dropped = 0;
    for (date, count) in pairs {
        match (date.parse::<DateKey>(), count.and_then(|c| u32::try_from(c).ok())) {
            (Ok(day), Some(n)) if n > 0 => {
                log.insert(day, n);
            }
            _ => dropped += 1,
        }
    }
    (log, dropped)
}

/// Adds one reviewed card to `today`.
pub fn record_review(log: &mut DayCounts, today: DateKey) {
    *log.entry(today).or_default() += 1;
}

/// Colour step for a count: 0 (none) to 4 (most). Same thresholds as the React app.
pub fn level(count: u32) -> usize {
    match count {
        0 => 0,
        1 => 1,
        2 => 2,
        3 | 4 => 3,
        _ => 4,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeatCell {
    pub day: DateKey,
    pub count: u32,
    /// After today; drawn as an empty slot.
    pub future: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heatmap {
    /// One column per week, Sunday first.
    pub weeks: Vec<[HeatCell; 7]>,
    /// `(column, month)` where a new month starts, for the labels on top.
    pub month_marks: Vec<(usize, u8)>,
    pub total: u32,
    pub active_days: usize,
}

impl Heatmap {
    /// The last `weeks` weeks, ending with the week that contains `today`.
    pub fn build(counts: &DayCounts, today: DateKey, weeks: usize) -> Self {
        let weeks = weeks.max(1);
        let mut start = today;
        for _ in 0..(today.weekday() as usize + (weeks - 1) * 7) {
            start = start.prev();
        }

        let mut columns = Vec::with_capacity(weeks);
        let mut month_marks = Vec::new();
        let mut total = 0;
        let mut active_days = 0;
        let mut day = start;
        for col in 0..weeks {
            if month_marks.last().is_none_or(|&(_, m)| m != day.month()) {
                month_marks.push((col, day.month()));
            }
            let week = std::array::from_fn(|_| {
                let future = day > today;
                let count = if future { 0 } else { counts.get(&day).copied().unwrap_or(0) };
                if count > 0 {
                    total += count;
                    active_days += 1;
                }
                let cell = HeatCell { day, count, future };
                day = day.next();
                cell
            });
            columns.push(week);
        }
        Self { weeks: columns, month_marks, total, active_days }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    fn d(s: &str) -> DateKey {
        s.parse().unwrap()
    }

    fn entry(date: &str) -> DailyEntry {
        let draft = EntryDraft { japanese: "x".into(), translation: "y".into(), ..Default::default() };
        draft.build(None, || date.into(), d(date), 0).unwrap()
    }

    #[test]
    fn counts_words_per_day() {
        let list = [entry("2026-11-10"), entry("2026-11-11"), entry("2026-11-11")];
        let counts = words_per_day(&list);
        assert_eq!(counts.get(&d("2026-11-11")), Some(&2));
        assert_eq!(counts.get(&d("2026-11-10")), Some(&1));
        assert_eq!(counts.len(), 2);
    }

    #[test]
    fn review_log_drops_bad_pairs() {
        let (log, dropped) = review_log_from_pairs([
            ("2026-11-11", Some(3)),
            ("2026-11-10", Some(0)),
            ("not-a-date", Some(2)),
            ("2026-11-09", None),
        ]);
        assert_eq!(log.len(), 1);
        assert_eq!(log[&d("2026-11-11")], 3);
        assert_eq!(dropped, 3);
    }

    #[test]
    fn records_reviews() {
        let mut log = DayCounts::new();
        record_review(&mut log, d("2026-11-11"));
        record_review(&mut log, d("2026-11-11"));
        assert_eq!(log[&d("2026-11-11")], 2);
    }

    #[test]
    fn levels_match_reference() {
        assert_eq!([0, 1, 2, 3, 4, 5, 40].map(level), [0, 1, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn heatmap_layout() {
        // Today is Wednesday 2026-11-11.
        let today = d("2026-11-11");
        let mut counts = DayCounts::new();
        counts.insert(d("2026-11-11"), 2);
        counts.insert(d("2026-11-02"), 1);
        counts.insert(d("2025-01-01"), 9); // outside the window
        counts.insert(d("2026-11-12"), 5); // future, ignored

        let map = Heatmap::build(&counts, today, 53);
        assert_eq!(map.weeks.len(), 53);
        let last = map.weeks.last().unwrap();
        assert_eq!(last[0].day, d("2026-11-08")); // Sunday of this week
        assert_eq!(last[3], HeatCell { day: today, count: 2, future: false });
        assert!(last[4].future && last[6].future);
        assert_eq!(map.weeks[0][0].day, d("2025-11-09"));
        assert_eq!(map.total, 3);
        assert_eq!(map.active_days, 2);

        // First mark is the first column; marks are in order with no repeats in a row.
        assert_eq!(map.month_marks[0], (0, 11));
        assert!(map.month_marks.windows(2).all(|w| w[0].0 < w[1].0 && w[0].1 != w[1].1));
        assert_eq!(map.month_marks.last(), Some(&(51, 11)));
    }
}
