//! App-wide reactive state, provided once via context.

use std::time::Duration;

use leptos::prelude::*;
use sabi_core::activity::{self, DayCounts};
use sabi_core::review::{Mark, ReviewSession};
use sabi_core::srs::{self, Rating, Schedule};
use sabi_core::model::{self, DraftError};
use sabi_core::settings::{Settings, Theme};
use sabi_core::{DailyEntry, DailyLimit, DateKey, EntryDraft, StreakStats, tags};

use crate::services::{clock, ids, storage, theme};

const SETTINGS_KEY: &str = "sabi.settings.v1";
const ENTRIES_KEY: &str = "sabi.entries.v1";
const CUSTOM_TAGS_KEY: &str = "sabi.customTags.v1";
const REVIEW_LOG_KEY: &str = "sabi.reviewLog.v1";
const REVIEW_SESSION_KEY: &str = "sabi.reviewSession.v1";
const SRS_KEY: &str = "sabi.srs.v1";

#[derive(Clone, Copy)]
pub struct AppState {
    pub settings: RwSignal<Settings>,
    /// Whether dark mode is currently showing (theme setting + OS preference).
    pub is_dark: Memo<bool>,
    pub entries: RwSignal<Vec<DailyEntry>>,
    /// User-created tags offered in the tag picker, besides the presets.
    pub custom_tags: RwSignal<Vec<String>>,
    /// Id of the entry open in the edit form on the Today page.
    pub editing: RwSignal<Option<String>>,
    /// Local date; rolls over at midnight while the app is open.
    pub today: RwSignal<DateKey>,
    pub stats: Memo<StreakStats>,
    /// Flashcards reviewed per day (filled by the Review page).
    pub review_log: RwSignal<DayCounts>,
    /// Today's flashcard progress; starts over at local midnight.
    pub review: RwSignal<ReviewSession>,
    /// Whether the Backup window is open.
    pub backup_open: RwSignal<bool>,
    /// SM-2 schedule per word id.
    pub srs: RwSignal<Schedule>,
    /// Words due by the end of today.
    pub due_count: Memo<usize>,
    /// Bumped to fire confetti.
    pub celebrate: RwSignal<u32>,
}

impl AppState {
    pub fn load() -> Self {
        let settings = RwSignal::new(storage::load::<Settings>(SETTINGS_KEY));
        let system_dark = theme::system_prefers_dark();
        let is_dark = Memo::new(move |_| settings.with(|s| s.theme.is_dark(system_dark)));

        let mut loaded: Vec<DailyEntry> = storage::load_list(ENTRIES_KEY);
        for e in &mut loaded {
            e.tags = tags::normalize_all(&e.tags);
        }
        let entries = RwSignal::new(loaded);
        let custom_tags = RwSignal::new(tags::normalize_all(&storage::load_list::<String>(CUSTOM_TAGS_KEY)));
        let today = RwSignal::new(clock::today());
        let stats = Memo::new(move |_| {
            let day = today.get();
            entries.with(|list| StreakStats::compute(list, day))
        });

        Effect::new(move |_| settings.with(|s| storage::save(SETTINGS_KEY, s)));
        Effect::new(move |_| entries.with(|list| storage::save(ENTRIES_KEY, list)));
        Effect::new(move |_| custom_tags.with(|t| storage::save(CUSTOM_TAGS_KEY, t)));
        let review_log = RwSignal::new(storage::load_day_counts(REVIEW_LOG_KEY));
        // Skip the first run so opening the app doesn't create an empty log.
        Effect::new(move |prev: Option<()>| {
            review_log.with(|log| {
                if prev.is_some() {
                    storage::save(REVIEW_LOG_KEY, log);
                }
            });
        });

        let saved_review = storage::load::<Option<ReviewSession>>(REVIEW_SESSION_KEY);
        let review = RwSignal::new(saved_review.unwrap_or_default().for_today(today.get_untracked()));
        Effect::new(move |_| review.with(|r| storage::save(REVIEW_SESSION_KEY, r)));

        let mut schedule: Schedule = storage::load_map(SRS_KEY);
        entries.with_untracked(|list| srs::prune(&mut schedule, list));
        let srs_signal = RwSignal::new(schedule);
        Effect::new(move |_| srs_signal.with(|s| storage::save(SRS_KEY, s)));
        let due_count = Memo::new(move |_| {
            let cutoff = clock::start_of_day_ms(today.get().next());
            entries.with(|list| srs_signal.with(|s| srs::due_count(list, s, cutoff)))
        });

        set_interval(
            move || {
                let now = clock::today();
                if today.get_untracked() != now {
                    today.set(now);
                    review.update(|r| *r = std::mem::take(r).for_today(now));
                }
            },
            Duration::from_secs(60),
        );

        Self { settings, is_dark, entries, custom_tags, editing: RwSignal::new(None), today, stats, review_log, review,
            backup_open: RwSignal::new(false),
            srs: srs_signal,
            due_count,
            celebrate: RwSignal::new(0),
        }
    }

    /// Keeps the `dark` class on <html> in sync with the theme.
    pub fn sync_theme(self) {
        Effect::new(move |_| theme::apply_dark_class(self.is_dark.get()));
    }

    pub fn toggle_theme(self) {
        let next = if self.is_dark.get_untracked() { Theme::Light } else { Theme::Dark };
        self.settings.update(|s| s.theme = next);
    }

    pub fn daily_limit(self) -> DailyLimit {
        self.settings.with(|s| s.daily_limit)
    }

    pub fn set_daily_limit(self, limit: DailyLimit) {
        self.settings.update(|s| s.daily_limit = limit);
    }

    pub fn find(self, id: &str) -> Option<DailyEntry> {
        self.entries.with_untracked(|list| list.iter().find(|e| e.id == id).cloned())
    }

    /// Saves a new entry (dated today) or an edit of `original_id`.
    pub fn save_draft(self, draft: &EntryDraft, original_id: Option<&str>) -> Result<DailyEntry, DraftError> {
        let original = original_id.and_then(|id| self.find(id));
        let entry = draft.build(original.as_ref(), ids::new_id, self.today.get_untracked(), clock::now_ms())?;

        let new_tags = self.custom_tags.with_untracked(|saved| tags::new_custom_tags(&entry.tags, saved));
        if !new_tags.is_empty() {
            self.custom_tags.update(|saved| saved.extend(new_tags));
        }
        self.entries.update(|list| model::upsert(list, entry.clone()));
        Ok(entry)
    }

    pub fn delete(self, id: &str) {
        self.entries.update(|list| list.retain(|e| e.id != id));
        self.srs.update(|s| {
            s.remove(id);
        });
        if self.editing.get_untracked().as_deref() == Some(id) {
            self.editing.set(None);
        }
    }

    pub fn toggle_favorite(self, id: &str) {
        self.entries.update(|list| {
            if let Some(e) = list.iter_mut().find(|e| e.id == id) {
                e.is_favorite = !e.is_favorite;
                e.updated_at = clock::now_ms();
            }
        });
    }

    /// Marks a flashcard and counts it in today's review log.
    pub fn mark_card(self, id: &str, mark: Mark, deck_len: usize) {
        self.review.update(|r| {
            r.mark(id, mark, deck_len);
            r.last_updated = clock::now_ms();
        });
        let day = self.today.get_untracked();
        self.review_log.update(|log| activity::record_review(log, day));
    }

    /// Answers a Due card: updates its SM-2 schedule and the review log.
    pub fn rate_card(self, id: &str, rating: Rating) {
        let now = clock::now_ms();
        self.srs.update(|s| {
            let next = s.get(id).cloned().unwrap_or_default().review(rating, now);
            s.insert(id.to_owned(), next);
        });
        let day = self.today.get_untracked();
        self.review_log.update(|log| activity::record_review(log, day));
    }

    /// Fires confetti (unless the user prefers reduced motion).
    pub fn fire_confetti(self) {
        self.celebrate.update(|n| *n += 1);
    }

    /// Adds imported words; a word with the same id is replaced.
    pub fn import(self, incoming: Vec<DailyEntry>) {
        let all_tags: Vec<String> = incoming.iter().flat_map(|e| e.tags.iter().cloned()).collect();
        // normalize_all also removes duplicates across entries.
        let new_tags = self.custom_tags.with_untracked(|saved| tags::normalize_all(&tags::new_custom_tags(&all_tags, saved)));
        if !new_tags.is_empty() {
            self.custom_tags.update(|saved| saved.extend(new_tags));
        }
        self.entries.update(|list| sabi_core::import::merge(list, incoming));
    }

    /// Deletes all words, custom tags and review history. Settings stay.
    pub fn reset_all(self) {
        self.editing.set(None);
        self.entries.set(Vec::new());
        self.custom_tags.set(Vec::new());
        self.review.set(ReviewSession::new(self.today.get_untracked()));
        self.review_log.set(DayCounts::new());
        self.srs.set(Schedule::new());
        for key in [ENTRIES_KEY, CUSTOM_TAGS_KEY, REVIEW_LOG_KEY, SRS_KEY] {
            storage::remove(&format!("{key}.backup"));
        }
    }

    /// Removes a tag from the saved list. Entries keep it.
    pub fn forget_custom_tag(self, tag: &str) {
        self.custom_tags.update(|saved| saved.retain(|t| t != tag));
    }
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}
