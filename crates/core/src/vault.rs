//! Search and filters for the Word Vault.

use crate::model::DailyEntry;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VaultFilter {
    pub query: String,
    pub tag: Option<String>,
    pub favorites_only: bool,
}

impl VaultFilter {
    pub fn is_active(&self) -> bool {
        !self.query.trim().is_empty() || self.tag.is_some() || self.favorites_only
    }

    /// Case-insensitive match on any text field; every filter must match.
    pub fn matches(&self, e: &DailyEntry) -> bool {
        if self.favorites_only && !e.is_favorite {
            return false;
        }
        if let Some(tag) = &self.tag
            && !e.tags.contains(tag)
        {
            return false;
        }
        let q = self.query.trim().to_lowercase();
        if q.is_empty() {
            return true;
        }
        let ex = e.example_sentence.as_ref();
        [
            Some(e.japanese.as_str()),
            Some(e.reading.as_str()),
            e.romaji.as_deref(),
            Some(e.translation.as_str()),
            e.notes.as_deref(),
            ex.map(|x| x.japanese.as_str()),
            ex.map(|x| x.translation.as_str()),
        ]
        .into_iter()
        .flatten()
        .chain(e.tags.iter().map(String::as_str))
        .any(|field| field.to_lowercase().contains(&q))
    }

    pub fn apply<'a>(&self, entries: &'a [DailyEntry]) -> Vec<&'a DailyEntry> {
        entries.iter().filter(|e| self.matches(e)).collect()
    }
}

/// Every tag used in the journal, most used first, then alphabetical.
pub fn tag_counts(entries: &[DailyEntry]) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    for tag in entries.iter().flat_map(|e| &e.tags) {
        match counts.iter_mut().find(|(t, _)| t == tag) {
            Some((_, n)) => *n += 1,
            None => counts.push((tag.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    fn entry(jp: &str, tr: &str, tags: &[&str], fav: bool) -> DailyEntry {
        let d = EntryDraft {
            japanese: jp.into(),
            translation: tr.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            is_favorite: fav,
            ..Default::default()
        };
        d.build(None, || jp.into(), "2026-11-11".parse().unwrap(), 0).unwrap()
    }

    fn list() -> Vec<DailyEntry> {
        vec![
            entry("木漏れ日", "Sunlight through leaves", &["Poetic", "Food & Nature"], true),
            entry("猫", "Cat", &["Daily Life"], false),
            entry("一期一会", "Once-in-a-lifetime meeting", &["Idiom (四字熟語)"], true),
        ]
    }

    fn ids(f: &VaultFilter) -> Vec<String> {
        f.apply(&list()).into_iter().map(|e| e.japanese.clone()).collect()
    }

    #[test]
    fn empty_filter_matches_all() {
        assert_eq!(ids(&VaultFilter::default()).len(), 3);
        assert!(!VaultFilter::default().is_active());
    }

    #[test]
    fn query_is_case_insensitive_across_fields() {
        let f = |q: &str| VaultFilter { query: q.into(), ..Default::default() };
        assert_eq!(ids(&f("SUNLIGHT")), vec!["木漏れ日"]);
        assert_eq!(ids(&f("猫")), vec!["猫"]);
        assert_eq!(ids(&f("poetic")), vec!["木漏れ日"], "tags are searchable");
    }

    #[test]
    fn tag_and_favorite_filters_combine() {
        let f = VaultFilter { favorites_only: true, ..Default::default() };
        assert_eq!(ids(&f), vec!["木漏れ日", "一期一会"]);
        let f = VaultFilter { tag: Some("Daily Life".into()), favorites_only: true, ..Default::default() };
        assert!(ids(&f).is_empty());
    }

    #[test]
    fn tag_counts_sorted_by_use() {
        let mut l = list();
        l.push(entry("犬", "Dog", &["Daily Life"], false));
        let counts = tag_counts(&l);
        assert_eq!(counts[0], ("Daily Life".into(), 2));
        assert_eq!(counts.len(), 4);
    }
}
