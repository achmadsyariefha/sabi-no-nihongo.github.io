//! SM-2 spaced repetition, ported from the backend's `utils/srs.ts`.
//!
//! Pass: 1 day, then 6 days, then `interval × ease`.
//! Fail: start over, ease −0.2 (never below 1.3), due again in 10 minutes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::model::DailyEntry;

pub const DAY_MS: i64 = 86_400_000;
pub const RELEARN_MS: i64 = 10 * 60 * 1000;
pub const START_EASE: f64 = 2.5;
pub const MIN_EASE: f64 = 1.3;
pub const FAIL_EASE_PENALTY: f64 = 0.2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rating {
    Pass,
    Fail,
}

/// One word's schedule. Words with no card yet are new and due now.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SrsCard {
    /// Days until the next review after a pass.
    pub interval: u32,
    pub ease: f64,
    pub repetitions: u32,
    pub lapses: u32,
    /// Milliseconds since the Unix epoch.
    pub due_at: i64,
    pub last_reviewed_at: Option<i64>,
}

impl Default for SrsCard {
    fn default() -> Self {
        Self { interval: 0, ease: START_EASE, repetitions: 0, lapses: 0, due_at: 0, last_reviewed_at: None }
    }
}

impl SrsCard {
    /// The schedule after answering at `now_ms`.
    pub fn review(&self, rating: Rating, now_ms: i64) -> Self {
        let mut next = self.clone();
        match rating {
            Rating::Fail => {
                next.repetitions = 0;
                next.interval = 0;
                next.lapses += 1;
                next.ease = (self.ease - FAIL_EASE_PENALTY).max(MIN_EASE);
                next.due_at = now_ms + RELEARN_MS;
            }
            Rating::Pass => {
                next.repetitions += 1;
                next.interval = match next.repetitions {
                    1 => 1,
                    2 => 6,
                    _ => (f64::from(self.interval) * self.ease).round() as u32,
                };
                next.due_at = now_ms + i64::from(next.interval) * DAY_MS;
            }
        }
        next.last_reviewed_at = Some(now_ms);
        next
    }
}

/// Schedules by entry id.
pub type Schedule = BTreeMap<String, SrsCard>;

/// Words due before `cutoff_ms` (usually the end of today). Order: cards due
/// right now first (reviews before new words, oldest due first), then cards
/// that come back later today, such as one just failed.
pub fn due_queue<'a>(entries: &'a [DailyEntry], schedule: &Schedule, now_ms: i64, cutoff_ms: i64) -> Vec<&'a DailyEntry> {
    let mut due: Vec<(&DailyEntry, Option<&SrsCard>)> = entries
        .iter()
        .map(|e| (e, schedule.get(&e.id)))
        .filter(|(_, card)| card.is_none_or(|c| c.due_at < cutoff_ms))
        .collect();
    due.sort_by_key(|(e, card)| {
        let due_at = card.map_or(0, |c| c.due_at);
        (due_at > now_ms, card.is_none(), due_at, e.date, e.created_at)
    });
    due.into_iter().map(|(e, _)| e).collect()
}

/// How many words are due before `cutoff_ms`.
pub fn due_count(entries: &[DailyEntry], schedule: &Schedule, cutoff_ms: i64) -> usize {
    entries.iter().filter(|e| schedule.get(&e.id).is_none_or(|c| c.due_at < cutoff_ms)).count()
}

/// When the earliest word becomes due.
pub fn next_due(entries: &[DailyEntry], schedule: &Schedule) -> Option<i64> {
    entries.iter().map(|e| schedule.get(&e.id).map_or(0, |c| c.due_at)).min()
}

/// Cards reviewed at or after `since_ms`.
pub fn reviewed_since(schedule: &Schedule, since_ms: i64) -> usize {
    schedule.values().filter(|c| c.last_reviewed_at.is_some_and(|t| t >= since_ms)).count()
}

/// Drops schedules for words that no longer exist.
pub fn prune(schedule: &mut Schedule, entries: &[DailyEntry]) {
    schedule.retain(|id, _| entries.iter().any(|e| &e.id == id));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    const NOW: i64 = 1_800_000_000_000;

    fn entry(id: &str, date: &str) -> DailyEntry {
        let draft = EntryDraft { japanese: id.into(), translation: "t".into(), ..Default::default() };
        draft.build(None, || id.into(), date.parse().unwrap(), 0).unwrap()
    }

    #[test]
    fn passes_follow_sm2_intervals() {
        let mut card = SrsCard::default();
        let mut intervals = Vec::new();
        for _ in 0..4 {
            card = card.review(Rating::Pass, NOW);
            intervals.push(card.interval);
        }
        // 1, 6, round(6 × 2.5) = 15, round(15 × 2.5) = 38
        assert_eq!(intervals, [1, 6, 15, 38]);
        assert_eq!(card.due_at, NOW + 38 * DAY_MS);
        assert_eq!(card.last_reviewed_at, Some(NOW));
        assert_eq!(card.ease, START_EASE);
    }

    #[test]
    fn fail_resets_and_lowers_ease() {
        let card = SrsCard::default().review(Rating::Pass, NOW).review(Rating::Pass, NOW);
        let failed = card.review(Rating::Fail, NOW);
        assert_eq!((failed.repetitions, failed.interval, failed.lapses), (0, 0, 1));
        assert!((failed.ease - 2.3).abs() < 1e-9);
        assert_eq!(failed.due_at, NOW + RELEARN_MS);
        assert_eq!(failed.review(Rating::Pass, NOW).interval, 1);

        let mut low = SrsCard { ease: 1.4, ..SrsCard::default() };
        for _ in 0..3 {
            low = low.review(Rating::Fail, NOW);
        }
        assert_eq!(low.ease, MIN_EASE);
    }

    #[test]
    fn queue_puts_reviews_first_and_relearning_last() {
        let list = [entry("new", "2026-11-01"), entry("old", "2026-10-01"), entry("later", "2026-10-02"), entry("failed", "2026-10-03")];
        let mut s = Schedule::new();
        s.insert("old".into(), SrsCard { due_at: NOW - DAY_MS, repetitions: 1, ..Default::default() });
        s.insert("later".into(), SrsCard { due_at: NOW + 5 * DAY_MS, ..Default::default() });
        s.insert("failed".into(), SrsCard { due_at: NOW + RELEARN_MS, ..Default::default() });
        let cutoff = NOW + DAY_MS / 2;
        let ids: Vec<&str> = due_queue(&list, &s, NOW, cutoff).iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["old", "new", "failed"]);
        assert_eq!(due_count(&list, &s, cutoff), 3);
        assert_eq!(next_due(&list[2..3], &s), Some(NOW + 5 * DAY_MS));
    }

    #[test]
    fn counts_reviews_and_prunes() {
        let list = [entry("a", "2026-11-01")];
        let mut s = Schedule::new();
        s.insert("a".into(), SrsCard::default().review(Rating::Pass, NOW));
        s.insert("gone".into(), SrsCard::default().review(Rating::Fail, NOW - DAY_MS));
        assert_eq!(reviewed_since(&s, NOW - 1000), 1);
        prune(&mut s, &list);
        assert_eq!(s.keys().collect::<Vec<_>>(), ["a"]);
    }

    #[test]
    fn serde_uses_camel_case_and_defaults() {
        let card: SrsCard = serde_json::from_str(r#"{"dueAt":5,"interval":6}"#).unwrap();
        assert_eq!((card.due_at, card.interval, card.ease), (5, 6, START_EASE));
        let json = serde_json::to_value(SrsCard::default().review(Rating::Pass, 7)).unwrap();
        assert_eq!(json["lastReviewedAt"], 7);
    }
}
