//! Streaks and simple counts derived from the journal.

use std::collections::BTreeSet;

use crate::DateKey;
use crate::model::DailyEntry;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreakStats {
    /// Consecutive logged days ending today, or yesterday if today isn't logged yet.
    pub current: u32,
    pub longest: u32,
    pub total_entries: usize,
    pub favorites: usize,
    pub today_count: usize,
    pub logged_days: BTreeSet<DateKey>,
}

impl StreakStats {
    pub fn compute(entries: &[DailyEntry], today: DateKey) -> Self {
        let logged_days: BTreeSet<DateKey> = entries.iter().map(|e| e.date).collect();

        let mut longest = 0;
        let mut run = 0;
        let mut prev: Option<DateKey> = None;
        for &d in &logged_days {
            run = if prev.is_some_and(|p| p.next() == d) { run + 1 } else { 1 };
            longest = longest.max(run);
            prev = Some(d);
        }

        let mut current = 0;
        let mut cursor = if logged_days.contains(&today) { Some(today) } else { Some(today.prev()) };
        while let Some(d) = cursor.filter(|d| logged_days.contains(d)) {
            current += 1;
            cursor = Some(d.prev());
        }

        Self {
            current,
            longest,
            total_entries: entries.len(),
            favorites: entries.iter().filter(|e| e.is_favorite).count(),
            today_count: entries.iter().filter(|e| e.date == today).count(),
            logged_days,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    fn entry(date: &str, fav: bool) -> DailyEntry {
        let d = EntryDraft { japanese: "x".into(), translation: "y".into(), is_favorite: fav, ..Default::default() };
        d.build(None, || date.into(), date.parse().unwrap(), 0).unwrap()
    }

    fn stats(dates: &[&str], today: &str) -> StreakStats {
        let list: Vec<_> = dates.iter().map(|d| entry(d, false)).collect();
        StreakStats::compute(&list, today.parse().unwrap())
    }

    #[test]
    fn empty_journal() {
        let s = stats(&[], "2026-11-11");
        assert_eq!((s.current, s.longest, s.total_entries, s.today_count), (0, 0, 0, 0));
    }

    #[test]
    fn current_streak_counts_back_from_today() {
        let s = stats(&["2026-11-09", "2026-11-10", "2026-11-11", "2026-11-11"], "2026-11-11");
        assert_eq!(s.current, 3);
        assert_eq!(s.today_count, 2);
        assert_eq!(s.total_entries, 4);
    }

    #[test]
    fn streak_survives_until_today_is_logged() {
        assert_eq!(stats(&["2026-11-09", "2026-11-10"], "2026-11-11").current, 2);
    }

    #[test]
    fn streak_breaks_after_a_missed_day() {
        assert_eq!(stats(&["2026-11-08", "2026-11-09"], "2026-11-11").current, 0);
    }

    #[test]
    fn longest_streak_crosses_month_and_year() {
        let s = stats(&["2025-12-30", "2025-12-31", "2026-01-01", "2026-01-03", "2026-02-28", "2026-03-01"], "2026-11-11");
        assert_eq!(s.longest, 3);
        assert_eq!(s.current, 0);
    }

    #[test]
    fn counts_favorites() {
        let list = vec![entry("2026-11-11", true), entry("2026-11-10", false), entry("2026-11-09", true)];
        assert_eq!(StreakStats::compute(&list, "2026-11-11".parse().unwrap()).favorites, 2);
    }
}
