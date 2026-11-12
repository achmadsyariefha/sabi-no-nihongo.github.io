//! Backup and export files: JSON (full backup, same shape as the React app),
//! CSV for spreadsheets, and a tab-separated file Anki can import.

use crate::model::DailyEntry;

/// Pretty JSON list of entries. Importable here and in the React app.
pub fn to_json(entries: &[DailyEntry]) -> String {
    serde_json::to_string_pretty(entries).expect("entries always serialize")
}

const CSV_HEADERS: [&str; 9] =
    ["Date", "Japanese", "Reading", "Translation", "Notes", "Tags", "IsFavorite", "Example Japanese", "Example Translation"];

fn csv_field(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// CSV with a UTF-8 byte order mark so Excel shows Japanese correctly.
pub fn to_csv(entries: &[DailyEntry]) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str(&CSV_HEADERS.join(","));
    for e in entries {
        let ex = e.example_sentence.as_ref();
        let row = [
            e.date.to_string(),
            e.japanese.clone(),
            e.reading.clone(),
            e.translation.clone(),
            e.notes.clone().unwrap_or_default(),
            e.tags.join(";"),
            if e.is_favorite { "Yes" } else { "No" }.to_owned(),
            ex.map(|x| x.japanese.clone()).unwrap_or_default(),
            ex.map(|x| x.translation.clone()).unwrap_or_default(),
        ];
        out.push('\n');
        out.push_str(&row.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
    }
    out.push('\n');
    out
}

/// Escapes text for Anki's HTML fields; tabs and newlines would break the row.
fn html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\t', " ")
        .replace("\r\n", "<br>")
        .replace('\n', "<br>")
}

/// Anki tags can't contain spaces.
fn anki_tag(t: &str) -> String {
    t.split_whitespace().collect::<Vec<_>>().join("_")
}

/// Tab-separated notes: front, back, tags. The header lines tell Anki
/// (2.1.54+) the format, so the import needs no settings.
pub fn to_anki_tsv(entries: &[DailyEntry]) -> String {
    let mut out = String::from("#separator:tab\n#html:true\n#tags column:3\n");
    for e in entries {
        let front = if e.reading.trim().is_empty() {
            html(&e.japanese)
        } else {
            format!("{} [{}]", html(&e.japanese), html(&e.reading))
        };
        let mut back = format!("<b>{}</b>", html(&e.translation));
        if let Some(notes) = &e.notes {
            back.push_str(&format!("<br><small>{}</small>", html(notes)));
        }
        if let Some(ex) = &e.example_sentence {
            back.push_str(&format!(
                "<hr><div style=\"font-size:0.9em\">{}<br><i>{}</i></div>",
                html(&ex.japanese),
                html(&ex.translation)
            ));
        }
        let tags: Vec<String> = e.tags.iter().map(|t| anki_tag(t)).collect();
        out.push_str(&format!("{front}\t{back}\t{}\n", tags.join(" ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;

    fn entry() -> DailyEntry {
        let draft = EntryDraft {
            japanese: "猫 \"ねこ\", <b>".into(),
            reading: "ねこ".into(),
            translation: "cat,\nfeline".into(),
            notes: "a\tb".into(),
            tags: vec!["Daily Life".into(), "JLPT N5".into()],
            example_japanese: "猫だ".into(),
            example_translation: "it's a cat".into(),
            is_favorite: true,
            ..Default::default()
        };
        draft.build(None, || "id".into(), "2026-11-11".parse().unwrap(), 0).unwrap()
    }

    #[test]
    fn csv_quotes_and_escapes() {
        let csv = to_csv(&[entry()]);
        assert!(csv.starts_with("\u{feff}Date,Japanese,"));
        assert!(csv.contains(r#""猫 ""ねこ"", <b>""#));
        assert!(csv.contains("\"cat,\nfeline\""));
        assert!(csv.contains(r#""Daily Life;JLPT N5","Yes","猫だ","it's a cat""#));
    }

    #[test]
    fn anki_rows_stay_on_one_line() {
        let tsv = to_anki_tsv(&[entry()]);
        let lines: Vec<&str> = tsv.lines().collect();
        assert_eq!(lines.len(), 4);
        let cols: Vec<&str> = lines[3].split('\t').collect();
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0], "猫 \"ねこ\", &lt;b&gt; [ねこ]");
        assert!(cols[1].starts_with("<b>cat,<br>feline</b><br><small>a b</small><hr>"));
        assert_eq!(cols[2], "Daily_Life JLPT_N5");
    }

    #[test]
    fn json_is_a_camel_case_list() {
        let json = to_json(&[entry()]);
        assert!(json.starts_with('['));
        assert!(json.contains("\"isFavorite\": true"));
        assert!(json.contains("\"exampleSentence\""));
    }
}
