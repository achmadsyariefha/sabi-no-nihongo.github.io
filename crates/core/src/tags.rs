//! Tag presets and cleanup rules.

pub const FAVORITE: &str = "Favorite";
const IDIOM_OLD: &str = "Idiom";
const IDIOM: &str = "Idiom (四字熟語)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TagInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
}

pub const PRESETS: [TagInfo; 9] = [
    TagInfo {
        name: FAVORITE,
        description: "Words or phrases you especially love and want to treasure in your collection.",
        hint: "Your top personal gems",
    },
    TagInfo {
        name: "Anime & Manga",
        description: "Lines, catchphrases, and vocabulary learned from watching anime or reading manga.",
        hint: "Anime dialogue & pop culture",
    },
    TagInfo {
        name: "Poetic",
        description: "Aesthetic, evocative expressions with seasonal or philosophical depth (e.g. 木漏れ日, 物の哀れ).",
        hint: "Aesthetic & seasonal expressions",
    },
    TagInfo {
        name: IDIOM,
        description: "Traditional 4-kanji proverbs (yojijukugo) that express timeless wisdom (e.g. 一期一会, 七転八起).",
        hint: "4-kanji wisdom & proverbs",
    },
    TagInfo {
        name: "Daily Life",
        description: "Practical words and greetings for everyday conversation, errands, work, transit, and routines.",
        hint: "Everyday conversations & greetings",
    },
    TagInfo {
        name: "Emotion",
        description: "Words for moods, feelings, nostalgia, and subtle sentiments (e.g. 懐かしい, 切ない).",
        hint: "Feelings, moods & nostalgia",
    },
    TagInfo {
        name: "Food & Nature",
        description: "Vocabulary for cuisine, seasons, weather, plants, animals, and scenery.",
        hint: "Food, weather & natural scenery",
    },
    TagInfo {
        name: "Slang & Casual",
        description: "Informal slang, colloquial speech, abbreviations, and casual spoken Japanese.",
        hint: "Informal colloquial expressions",
    },
    TagInfo {
        name: "JLPT N5-N1",
        description: "Test vocabulary and grammar from beginner (N5) to advanced (N1).",
        hint: "Standard proficiency test vocabulary",
    },
];

pub fn is_preset(tag: &str) -> bool {
    PRESETS.iter().any(|p| p.name == tag)
}

pub fn preset_info(tag: &str) -> Option<&'static TagInfo> {
    PRESETS.iter().find(|p| p.name == tag)
}

/// Strips leading `#`, trims, and migrates the old `Idiom` tag. `None` if empty.
pub fn normalize(tag: &str) -> Option<String> {
    let t = tag.trim().trim_start_matches('#').trim();
    match t {
        "" => None,
        IDIOM_OLD => Some(IDIOM.to_owned()),
        _ => Some(t.to_owned()),
    }
}

/// Normalizes every tag and removes duplicates, keeping first-seen order.
pub fn normalize_all<S: AsRef<str>>(tags: &[S]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(tags.len());
    for tag in tags {
        if let Some(t) = normalize(tag.as_ref())
            && !out.contains(&t)
        {
            out.push(t);
        }
    }
    out
}

/// Tags in `tags` that are neither presets nor already saved as custom tags.
pub fn new_custom_tags(tags: &[String], saved: &[String]) -> Vec<String> {
    tags.iter().filter(|t| !is_preset(t) && !saved.contains(t)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_hash_and_migrates_idiom() {
        assert_eq!(normalize("  ##Anime  "), Some("Anime".into()));
        assert_eq!(normalize("Idiom"), Some("Idiom (四字熟語)".into()));
        assert_eq!(normalize(" # "), None);
    }

    #[test]
    fn normalize_all_dedupes_in_order() {
        assert_eq!(normalize_all(&["b", "#a", "b", "a", ""]), vec!["b", "a"]);
    }

    #[test]
    fn finds_new_custom_tags() {
        let tags = vec!["Favorite".to_owned(), "Ghibli".to_owned(), "Saved".to_owned()];
        assert_eq!(new_custom_tags(&tags, &["Saved".to_owned()]), vec!["Ghibli"]);
    }
}
