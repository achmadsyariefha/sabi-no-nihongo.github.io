//! Reading backup files: our own JSON export, a backup from the React app,
//! or a raw copy of `sabi.entries.v1`.

use serde_json::Value;

use crate::DateKey;
use crate::model::{DailyEntry, ExampleSentence};
use crate::tags;

#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub entries: Vec<DailyEntry>,
    pub skipped: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportError {
    NotJson,
    NotAList,
}

impl ImportError {
    pub fn message(self) -> &'static str {
        match self {
            ImportError::NotJson => "This file isn't valid JSON.",
            ImportError::NotAList => "This file doesn't look like a word backup (expected a list of words).",
        }
    }
}

/// Parses a backup file's text. `now_ms` fills missing timestamps.
pub fn parse(text: &str, now_ms: i64) -> Result<Imported, ImportError> {
    let value: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|_| ImportError::NotJson)?;
    let items = match &value {
        Value::Array(items) => items,
        // A copy of our localStorage value: { "v": 1, "data": [...] }
        Value::Object(map) => match map.get("data") {
            Some(Value::Array(items)) => items,
            _ => return Err(ImportError::NotAList),
        },
        _ => return Err(ImportError::NotAList),
    };
    Ok(normalize(items, now_ms))
}

/// Keeps items with an id, Japanese text and a valid date; coerces the rest.
pub fn normalize(items: &[Value], now_ms: i64) -> Imported {
    let entries: Vec<DailyEntry> = items.iter().filter_map(|item| entry_from(item, now_ms)).collect();
    Imported { skipped: items.len() - entries.len(), entries }
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_owned()
}

fn optional(v: &Value, key: &str) -> Option<String> {
    Some(text(v, key)).filter(|s| !s.trim().is_empty())
}

fn timestamp(v: &Value, key: &str) -> Option<i64> {
    let n = v.get(key)?;
    n.as_i64().or_else(|| n.as_f64().filter(|f| f.is_finite()).map(|f| f as i64))
}

fn entry_from(v: &Value, now_ms: i64) -> Option<DailyEntry> {
    let id = text(v, "id").trim().to_owned();
    let japanese = text(v, "japanese").trim().to_owned();
    let date: DateKey = text(v, "date").parse().ok()?;
    if id.is_empty() || japanese.is_empty() {
        return None;
    }
    let raw_tags: Vec<String> = v
        .get("tags")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).map(str::to_owned).collect())
        .unwrap_or_default();
    let example_sentence = v.get("exampleSentence").filter(|ex| ex.is_object()).and_then(|ex| {
        optional(ex, "japanese").map(|japanese| ExampleSentence {
            japanese,
            reading: optional(ex, "reading"),
            translation: text(ex, "translation"),
        })
    });
    let created_at = timestamp(v, "createdAt").unwrap_or(now_ms);
    Some(DailyEntry {
        id,
        date,
        japanese,
        reading: text(v, "reading"),
        romaji: optional(v, "romaji"),
        translation: text(v, "translation"),
        notes: optional(v, "notes"),
        tags: tags::normalize_all(&raw_tags),
        is_favorite: v.get("isFavorite").and_then(Value::as_bool).unwrap_or(false),
        example_sentence,
        created_at,
        updated_at: timestamp(v, "updatedAt").unwrap_or(created_at),
    })
}

/// How an import would change the journal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MergePlan {
    pub added: usize,
    pub replaced: usize,
}

pub fn plan_merge(existing: &[DailyEntry], incoming: &[DailyEntry]) -> MergePlan {
    let replaced = incoming.iter().filter(|e| existing.iter().any(|x| x.id == e.id)).count();
    MergePlan { added: incoming.len() - replaced, replaced }
}

/// Adds `incoming` to `existing`; an entry with the same id is replaced.
pub fn merge(existing: &mut Vec<DailyEntry>, incoming: Vec<DailyEntry>) {
    for e in incoming {
        crate::model::upsert(existing, e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Missing timestamps become 0 in tests.
    fn parse(text: &str) -> Result<Imported, ImportError> {
        super::parse(text, 0)
    }

    #[test]
    fn reads_a_react_backup() {
        let json = r##"[
          {"id":"1","date":"2026-05-01","japanese":" 木漏れ日 ","reading":"こもれび","translation":"sunlight",
           "notes":"","tags":["#Poetic","Idiom","Poetic"],"isFavorite":true,
           "exampleSentence":{"japanese":"森","translation":"forest"},"createdAt":1.7e12,"updatedAt":1700000000001},
          {"id":"2","date":"2026-02-30","japanese":"x"},
          {"id":"","date":"2026-05-01","japanese":"x"},
          {"id":"3","date":"2026-05-02","japanese":""},
          "junk",
          {"id":"4","date":"2026-05-03","japanese":"猫"}
        ]"##;
        let got = parse(json).unwrap();
        assert_eq!(got.skipped, 4);
        assert_eq!(got.entries.len(), 2);
        let a = &got.entries[0];
        assert_eq!(a.japanese, "木漏れ日");
        assert_eq!(a.notes, None);
        assert_eq!(a.tags, ["Poetic", "Idiom (四字熟語)"]);
        assert!(a.is_favorite);
        assert_eq!(a.created_at, 1_700_000_000_000);
        assert_eq!(a.example_sentence.as_ref().unwrap().reading, None);
        let b = &got.entries[1];
        assert_eq!((b.reading.as_str(), b.tags.len(), b.is_favorite, b.created_at, b.updated_at), ("", 0, false, 0, 0));
    }

    #[test]
    fn reads_a_storage_copy_and_rejects_other_shapes() {
        let wrapped = r#"{"v":1,"data":[{"id":"1","date":"2026-05-01","japanese":"猫","createdAt":5,"updatedAt":6}]}"#;
        assert_eq!(parse(wrapped).unwrap().entries.len(), 1);
        assert_eq!(parse("\u{feff}[]").unwrap().entries.len(), 0);
        assert_eq!(parse("{nope").unwrap_err(), ImportError::NotJson);
        assert_eq!(parse(r#"{"words":[]}"#).unwrap_err(), ImportError::NotAList);
        assert_eq!(parse("42").unwrap_err(), ImportError::NotAList);
    }

    #[test]
    fn round_trips_our_own_export() {
        let original = parse(
            r#"[{"id":"1","date":"2026-05-01","japanese":"猫","reading":"ねこ","romaji":"neko","translation":"cat",
                "notes":"n","tags":["Animals"],"isFavorite":true,
                "exampleSentence":{"japanese":"猫だ","reading":"ねこだ","translation":"a cat"},"createdAt":5,"updatedAt":6}]"#,
        )
        .unwrap()
        .entries;
        let again = parse(&crate::export::to_json(&original)).unwrap();
        assert_eq!(again.entries, original);
        assert_eq!(again.skipped, 0);
    }

    #[test]
    fn merge_replaces_by_id() {
        let mk = |id: &str, jp: &str| parse(&format!(r#"[{{"id":"{id}","date":"2026-05-01","japanese":"{jp}"}}]"#)).unwrap().entries;
        let mut journal = [mk("1", "old"), mk("2", "keep")].concat();
        let incoming = [mk("1", "new"), mk("3", "add")].concat();
        assert_eq!(plan_merge(&journal, &incoming), MergePlan { added: 1, replaced: 1 });
        merge(&mut journal, incoming);
        let words: Vec<&str> = journal.iter().map(|e| e.japanese.as_str()).collect();
        assert_eq!(words, ["new", "keep", "add"]);
    }
}
