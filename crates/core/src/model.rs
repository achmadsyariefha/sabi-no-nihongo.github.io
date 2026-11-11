//! Journal entries. JSON field names match the React app's backups (camelCase).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::DateKey;
use crate::tags;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyEntry {
    pub id: String,
    pub date: DateKey,
    pub japanese: String,
    #[serde(default)]
    pub reading: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub romaji: Option<String>,
    #[serde(default)]
    pub translation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example_sentence: Option<ExampleSentence>,
    /// Milliseconds since the Unix epoch.
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExampleSentence {
    pub japanese: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reading: Option<String>,
    #[serde(default)]
    pub translation: String,
}

/// Newest date first; within a day, newest created first.
pub fn sort_newest_first(entries: &mut [DailyEntry]) {
    entries.sort_by(|a, b| b.date.cmp(&a.date).then(b.created_at.cmp(&a.created_at)));
}

/// Entries for one day, oldest first (the order they were logged).
pub fn entries_on(entries: &[DailyEntry], day: DateKey) -> Vec<DailyEntry> {
    let mut list: Vec<DailyEntry> = entries.iter().filter(|e| e.date == day).cloned().collect();
    list.sort_by_key(|e| e.created_at);
    list
}

/// Inserts a new entry or replaces the one with the same id.
pub fn upsert(entries: &mut Vec<DailyEntry>, entry: DailyEntry) {
    match entries.iter_mut().find(|e| e.id == entry.id) {
        Some(existing) => *existing = entry,
        None => entries.push(entry),
    }
}

// ---------------------------------------------------------------- form draft

/// What the entry form edits. Plain strings, validated on save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryDraft {
    pub japanese: String,
    pub reading: String,
    pub romaji: String,
    pub translation: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub example_japanese: String,
    pub example_reading: String,
    pub example_translation: String,
}

impl Default for EntryDraft {
    /// New words start as a starred favorite, like the React app.
    fn default() -> Self {
        Self {
            japanese: String::new(),
            reading: String::new(),
            romaji: String::new(),
            translation: String::new(),
            notes: String::new(),
            tags: vec![tags::FAVORITE.to_owned()],
            is_favorite: true,
            example_japanese: String::new(),
            example_reading: String::new(),
            example_translation: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftError {
    MissingJapanese,
    MissingTranslation,
}

impl DraftError {
    pub fn message(self) -> &'static str {
        match self {
            DraftError::MissingJapanese => "Enter the Japanese word or sentence.",
            DraftError::MissingTranslation => "Enter a translation.",
        }
    }
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

impl EntryDraft {
    pub fn from_entry(e: &DailyEntry) -> Self {
        let ex = e.example_sentence.as_ref();
        Self {
            japanese: e.japanese.clone(),
            reading: e.reading.clone(),
            romaji: e.romaji.clone().unwrap_or_default(),
            translation: e.translation.clone(),
            notes: e.notes.clone().unwrap_or_default(),
            tags: e.tags.clone(),
            is_favorite: e.is_favorite,
            example_japanese: ex.map(|x| x.japanese.clone()).unwrap_or_default(),
            example_reading: ex.and_then(|x| x.reading.clone()).unwrap_or_default(),
            example_translation: ex.map(|x| x.translation.clone()).unwrap_or_default(),
        }
    }

    pub fn validate(&self) -> Result<(), DraftError> {
        if self.japanese.trim().is_empty() {
            return Err(DraftError::MissingJapanese);
        }
        if self.translation.trim().is_empty() {
            return Err(DraftError::MissingTranslation);
        }
        Ok(())
    }

    /// Builds the entry to save. For an edit, pass the original so its id,
    /// date and creation time are kept.
    pub fn build(
        &self,
        original: Option<&DailyEntry>,
        new_id: impl FnOnce() -> String,
        today: DateKey,
        now_ms: i64,
    ) -> Result<DailyEntry, DraftError> {
        self.validate()?;
        let mut tags = tags::normalize_all(&self.tags);
        if tags.is_empty() {
            tags.push(tags::FAVORITE.to_owned());
        }
        let example_sentence = non_empty(&self.example_japanese).map(|japanese| ExampleSentence {
            japanese,
            reading: non_empty(&self.example_reading),
            translation: self.example_translation.trim().to_owned(),
        });
        Ok(DailyEntry {
            id: original.map(|o| o.id.clone()).unwrap_or_else(new_id),
            date: original.map(|o| o.date).unwrap_or(today),
            japanese: self.japanese.trim().to_owned(),
            reading: self.reading.trim().to_owned(),
            romaji: non_empty(&self.romaji),
            translation: self.translation.trim().to_owned(),
            notes: non_empty(&self.notes),
            tags,
            is_favorite: self.is_favorite,
            example_sentence,
            created_at: original.map(|o| o.created_at).unwrap_or(now_ms),
            updated_at: now_ms,
        })
    }
}

// ---------------------------------------------------------------- daily goal

/// How many words per day count as "goal met". Not a hard limit: bonus words
/// can still be added. Stored as a number (0 = unlimited) like the React app.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DailyLimit {
    #[default]
    One,
    Two,
    Three,
    Five,
    Unlimited,
}

impl DailyLimit {
    pub const ALL: [DailyLimit; 5] = [Self::One, Self::Two, Self::Three, Self::Five, Self::Unlimited];

    pub fn count(self) -> Option<usize> {
        match self {
            Self::One => Some(1),
            Self::Two => Some(2),
            Self::Three => Some(3),
            Self::Five => Some(5),
            Self::Unlimited => None,
        }
    }

    pub fn from_number(n: u64) -> Option<Self> {
        match n {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            5 => Some(Self::Five),
            0 => Some(Self::Unlimited),
            _ => None,
        }
    }

    pub fn as_number(self) -> u64 {
        self.count().unwrap_or(0) as u64
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::One => "1 word / day",
            Self::Two => "2 words / day",
            Self::Three => "3 words / day",
            Self::Five => "5 words / day",
            Self::Unlimited => "No goal",
        }
    }

    pub fn is_met(self, logged_today: usize) -> bool {
        self.count().is_some_and(|n| logged_today >= n)
    }
}

impl Serialize for DailyLimit {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.as_number())
    }
}

impl<'de> Deserialize<'de> for DailyLimit {
    /// Unknown numbers fall back to the default instead of failing the whole settings object.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let n = u64::deserialize(d)?;
        Ok(Self::from_number(n).unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(s: &str) -> DateKey {
        s.parse().unwrap()
    }

    fn draft(jp: &str, tr: &str) -> EntryDraft {
        EntryDraft { japanese: jp.into(), translation: tr.into(), ..Default::default() }
    }

    #[test]
    fn reference_backup_round_trips() {
        let json = r#"{
            "id": "sample-1", "date": "2026-08-30", "japanese": "木漏れ日",
            "reading": "こもれび (komorebi)", "translation": "Sunlight filtering through leaves",
            "notes": "Poetic.", "tags": ["Favorite", "Nature"], "isFavorite": true,
            "exampleSentence": { "japanese": "森の中で木漏れ日を浴びた。", "translation": "Bathed in komorebi." },
            "createdAt": 1756512000000, "updatedAt": 1756512000000
        }"#;
        let e: DailyEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.date, day("2026-08-30"));
        assert!(e.is_favorite);
        assert_eq!(e.example_sentence.as_ref().unwrap().reading, None);
        let back = serde_json::to_value(&e).unwrap();
        assert_eq!(back["isFavorite"], true);
        assert_eq!(back["exampleSentence"]["japanese"], "森の中で木漏れ日を浴びた。");
        assert!(back.get("romaji").is_none(), "empty optionals are omitted");
    }

    #[test]
    fn draft_requires_japanese_and_translation() {
        assert_eq!(draft("  ", "x").validate(), Err(DraftError::MissingJapanese));
        assert_eq!(draft("猫", " ").validate(), Err(DraftError::MissingTranslation));
        assert!(draft("猫", "cat").validate().is_ok());
    }

    #[test]
    fn build_new_entry_trims_and_defaults() {
        let mut d = draft("  猫 ", " cat ");
        d.tags = vec!["#Animals".into(), "Animals".into(), " ".into()];
        d.example_japanese = "   ".into();
        let e = d.build(None, || "id-1".into(), day("2026-11-11"), 42).unwrap();
        assert_eq!(e.id, "id-1");
        assert_eq!(e.date, day("2026-11-11"));
        assert_eq!(e.japanese, "猫");
        assert_eq!(e.translation, "cat");
        assert_eq!(e.tags, vec!["Animals"]);
        assert_eq!(e.example_sentence, None, "blank example is dropped");
        assert_eq!(e.notes, None);
        assert_eq!((e.created_at, e.updated_at), (42, 42));
    }

    #[test]
    fn build_with_no_tags_uses_favorite() {
        let mut d = draft("猫", "cat");
        d.tags.clear();
        let e = d.build(None, || "x".into(), day("2026-11-11"), 1).unwrap();
        assert_eq!(e.tags, vec!["Favorite"]);
    }

    #[test]
    fn editing_keeps_id_date_and_created_at() {
        let orig = draft("猫", "cat").build(None, || "a".into(), day("2026-11-01"), 10).unwrap();
        let mut d = EntryDraft::from_entry(&orig);
        d.translation = "kitty".into();
        let edited = d.build(Some(&orig), || unreachable!(), day("2026-11-11"), 99).unwrap();
        assert_eq!(edited.id, "a");
        assert_eq!(edited.date, day("2026-11-01"));
        assert_eq!(edited.created_at, 10);
        assert_eq!(edited.updated_at, 99);
        assert_eq!(edited.translation, "kitty");
    }

    #[test]
    fn upsert_replaces_or_appends() {
        let a = draft("a", "a").build(None, || "1".into(), day("2026-11-11"), 1).unwrap();
        let mut list = vec![a.clone()];
        let mut a2 = a.clone();
        a2.translation = "changed".into();
        upsert(&mut list, a2);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].translation, "changed");
        let b = draft("b", "b").build(None, || "2".into(), day("2026-11-11"), 2).unwrap();
        upsert(&mut list, b);
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn daily_limit_numbers() {
        assert_eq!(serde_json::to_string(&DailyLimit::Five).unwrap(), "5");
        assert_eq!(serde_json::to_string(&DailyLimit::Unlimited).unwrap(), "0");
        assert_eq!(serde_json::from_str::<DailyLimit>("2").unwrap(), DailyLimit::Two);
        assert_eq!(serde_json::from_str::<DailyLimit>("4").unwrap(), DailyLimit::One);
        assert!(DailyLimit::Two.is_met(2));
        assert!(!DailyLimit::Two.is_met(1));
        assert!(!DailyLimit::Unlimited.is_met(100));
    }

    #[test]
    fn entries_on_day_are_in_logged_order() {
        let mk = |id: &str, d: &str, t: i64| draft("x", "y").build(None, || id.into(), day(d), t).unwrap();
        let list = vec![mk("late", "2026-11-11", 5), mk("other", "2026-11-10", 1), mk("early", "2026-11-11", 2)];
        let ids: Vec<_> = entries_on(&list, day("2026-11-11")).into_iter().map(|e| e.id).collect();
        assert_eq!(ids, vec!["early", "late"]);
    }
}
