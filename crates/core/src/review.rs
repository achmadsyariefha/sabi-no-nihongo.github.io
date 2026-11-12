//! The daily flashcard session: go through a deck and mark each card
//! "Known" or "Need review". Progress is kept for the day and starts over
//! at local midnight.

use serde::{Deserialize, Serialize};

use crate::DateKey;
use crate::model::DailyEntry;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewFilter {
    #[default]
    All,
    Favorites,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Known,
    NeedReview,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReviewSession {
    pub date_key: DateKey,
    pub filter: ReviewFilter,
    /// Position in the deck. `deck.len()` means the summary is showing.
    pub current_index: usize,
    pub known: Vec<String>,
    pub need_review: Vec<String>,
    /// Card ids in shuffled order; empty means the default order.
    pub order: Vec<String>,
    pub last_updated: i64,
}

impl Default for ReviewSession {
    fn default() -> Self {
        Self::new(DateKey::new(1970, 1, 1).expect("valid date"))
    }
}

impl ReviewSession {
    pub fn new(today: DateKey) -> Self {
        Self {
            date_key: today,
            filter: ReviewFilter::All,
            current_index: 0,
            known: Vec::new(),
            need_review: Vec::new(),
            order: Vec::new(),
            last_updated: 0,
        }
    }

    /// The saved session if it is from `today`, otherwise a fresh one that
    /// keeps the filter.
    pub fn for_today(self, today: DateKey) -> Self {
        if self.date_key == today { self } else { Self { filter: self.filter, ..Self::new(today) } }
    }

    /// Cards to review today. See [`deck`].
    pub fn deck<'a>(&self, entries: &'a [DailyEntry]) -> Vec<&'a DailyEntry> {
        deck(entries, self.filter, &self.order)
    }

    /// Index to show, clamped to the deck (the summary counts as one slot).
    pub fn index_in(&self, deck_len: usize) -> usize {
        self.current_index.min(deck_len)
    }

    pub fn set_filter(&mut self, filter: ReviewFilter) {
        if self.filter != filter {
            self.filter = filter;
            self.current_index = 0;
        }
    }

    pub fn go_to(&mut self, index: usize, deck_len: usize) {
        self.current_index = index.min(deck_len);
    }

    /// Marks a card and moves to the next one (or the summary).
    pub fn mark(&mut self, id: &str, mark: Mark, deck_len: usize) {
        self.known.retain(|k| k != id);
        self.need_review.retain(|k| k != id);
        match mark {
            Mark::Known => self.known.push(id.to_owned()),
            Mark::NeedReview => self.need_review.push(id.to_owned()),
        }
        self.current_index = (self.index_in(deck_len) + 1).min(deck_len);
    }

    pub fn mark_of(&self, id: &str) -> Option<Mark> {
        if self.known.iter().any(|k| k == id) {
            Some(Mark::Known)
        } else if self.need_review.iter().any(|k| k == id) {
            Some(Mark::NeedReview)
        } else {
            None
        }
    }

    /// `(known, need review)` among the cards in `deck`.
    pub fn counts(&self, deck: &[&DailyEntry]) -> (usize, usize) {
        let count = |ids: &[String]| deck.iter().filter(|e| ids.contains(&e.id)).count();
        (count(&self.known), count(&self.need_review))
    }

    /// Puts the deck in a new random order. `rand_below(n)` must return a
    /// number in `0..n`.
    pub fn shuffle(&mut self, deck: &[&DailyEntry], mut rand_below: impl FnMut(usize) -> usize) {
        let mut ids: Vec<String> = deck.iter().map(|e| e.id.clone()).collect();
        for i in (1..ids.len()).rev() {
            ids.swap(i, rand_below(i + 1));
        }
        self.order = ids;
        self.current_index = 0;
    }

    /// Clears today's marks and starts from the first card.
    pub fn restart(&mut self) {
        *self = Self { filter: self.filter, order: std::mem::take(&mut self.order), ..Self::new(self.date_key) };
    }
}

/// The filtered entries, oldest first, or in the shuffled `order`. Words
/// added after a shuffle go at the end.
pub fn deck<'a>(entries: &'a [DailyEntry], filter: ReviewFilter, order: &[String]) -> Vec<&'a DailyEntry> {
    let mut deck: Vec<&DailyEntry> = entries.iter().filter(|e| filter == ReviewFilter::All || e.is_favorite).collect();
    deck.sort_by(|a, b| a.date.cmp(&b.date).then(a.created_at.cmp(&b.created_at)));
    if !order.is_empty() {
        // Stable sort: ids missing from `order` keep date order at the end.
        deck.sort_by_key(|e| order.iter().position(|id| *id == e.id).unwrap_or(usize::MAX));
    }
    deck
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    fn d(s: &str) -> DateKey {
        s.parse().unwrap()
    }

    fn entry(id: &str, date: &str, created: i64, fav: bool) -> DailyEntry {
        let draft = EntryDraft { japanese: id.into(), translation: "t".into(), is_favorite: fav, ..Default::default() };
        draft.build(None, || id.into(), d(date), created).unwrap()
    }

    fn sample() -> Vec<DailyEntry> {
        vec![
            entry("c", "2026-11-11", 3, false),
            entry("a", "2026-11-09", 1, true),
            entry("b", "2026-11-10", 2, true),
        ]
    }

    fn ids(deck: &[&DailyEntry]) -> Vec<String> {
        deck.iter().map(|e| e.id.clone()).collect()
    }

    #[test]
    fn deck_is_oldest_first_and_filtered() {
        let list = sample();
        let mut s = ReviewSession::new(d("2026-11-11"));
        assert_eq!(ids(&s.deck(&list)), ["a", "b", "c"]);
        s.set_filter(ReviewFilter::Favorites);
        assert_eq!(ids(&s.deck(&list)), ["a", "b"]);
    }

    #[test]
    fn marking_moves_on_and_switches_sides() {
        let list = sample();
        let mut s = ReviewSession::new(d("2026-11-11"));
        let n = s.deck(&list).len();
        s.mark("a", Mark::Known, n);
        s.mark("b", Mark::NeedReview, n);
        assert_eq!(s.current_index, 2);
        s.go_to(1, n);
        s.mark("b", Mark::Known, n);
        assert_eq!(s.mark_of("b"), Some(Mark::Known));
        assert_eq!(s.counts(&s.deck(&list)), (2, 0));
        s.mark("c", Mark::NeedReview, n);
        s.mark("c", Mark::NeedReview, n);
        assert_eq!(s.index_in(n), n, "stays on the summary");
        assert_eq!(s.counts(&s.deck(&list)), (2, 1));
    }

    #[test]
    fn counts_ignore_deleted_cards() {
        let list = sample();
        let mut s = ReviewSession::new(d("2026-11-11"));
        s.known = vec!["a".into(), "gone".into()];
        assert_eq!(s.counts(&s.deck(&list)), (1, 0));
    }

    #[test]
    fn new_day_starts_over_but_keeps_filter() {
        let mut s = ReviewSession::new(d("2026-11-11"));
        s.set_filter(ReviewFilter::Favorites);
        s.mark("a", Mark::Known, 3);
        let same = s.clone().for_today(d("2026-11-11"));
        assert_eq!(same, s);
        let next = s.for_today(d("2026-11-12"));
        assert_eq!(next.date_key, d("2026-11-12"));
        assert_eq!(next.filter, ReviewFilter::Favorites);
        assert!(next.known.is_empty());
        assert_eq!(next.current_index, 0);
    }

    #[test]
    fn shuffle_keeps_order_and_puts_new_words_last() {
        let mut list = sample();
        let mut s = ReviewSession::new(d("2026-11-11"));
        let deck = s.deck(&list);
        // rand_below(n) = 0 rotates the deck: [a, b, c] -> [b, c, a].
        s.shuffle(&deck, |_| 0);
        assert_eq!(ids(&s.deck(&list)), ["b", "c", "a"]);
        list.push(entry("new", "2026-11-11", 9, false));
        assert_eq!(ids(&s.deck(&list)), ["b", "c", "a", "new"]);
        s.restart();
        assert_eq!(ids(&s.deck(&list)), ["b", "c", "a", "new"], "restart keeps the shuffle");
    }

    #[test]
    fn index_clamps_when_deck_shrinks() {
        let mut s = ReviewSession::new(d("2026-11-11"));
        s.current_index = 10;
        assert_eq!(s.index_in(2), 2);
        s.go_to(99, 3);
        assert_eq!(s.current_index, 3);
    }

    #[test]
    fn serde_uses_camel_case() {
        let mut s = ReviewSession::new(d("2026-11-11"));
        s.need_review.push("x".into());
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["dateKey"], "2026-11-11");
        assert_eq!(json["filter"], "all");
        assert_eq!(json["needReview"][0], "x");
        let back: ReviewSession = serde_json::from_value(json).unwrap();
        assert_eq!(back, s);
    }
}
