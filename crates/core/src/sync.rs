//! Sync between the browser and the optional server.
//!
//! The browser remembers what the server last confirmed (`SyncState`). Each
//! sync sends everything that differs from that (new, edited and deleted
//! words, and SM-2 cards) and gets back everything that changed on the server
//! since the last sync, plus the server's copy of everything it was sent.
//! Both sides keep whichever version has the newer timestamp
//! (`updatedAt` for words, `lastReviewedAt` for cards).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::model::{self, DailyEntry};
use crate::srs::{Schedule, SrsCard};

/// A word deleted at `deleted_at` (ms).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Deletion {
    pub id: String,
    pub deleted_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncRequest {
    /// From the previous response; 0 on the first sync.
    pub cursor: i64,
    pub entries: Vec<DailyEntry>,
    pub deleted: Vec<Deletion>,
    pub srs: BTreeMap<String, SrsCard>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncResponse {
    pub cursor: i64,
    pub entries: Vec<DailyEntry>,
    pub deleted: Vec<Deletion>,
    pub srs: BTreeMap<String, SrsCard>,
}

/// What the server is known to have, as of the last sync.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncState {
    pub cursor: i64,
    /// Word id → `updatedAt` the server has.
    pub entries: BTreeMap<String, i64>,
    /// Word id → `lastReviewedAt` (0 if never) the server has.
    pub srs: BTreeMap<String, i64>,
}

fn reviewed_at(card: &SrsCard) -> i64 {
    card.last_reviewed_at.unwrap_or(0)
}

/// Everything that differs from what the server last confirmed.
pub fn outgoing(entries: &[DailyEntry], srs: &Schedule, state: &SyncState, now_ms: i64) -> SyncRequest {
    let local_ids: BTreeSet<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    SyncRequest {
        cursor: state.cursor,
        entries: entries.iter().filter(|e| state.entries.get(&e.id) != Some(&e.updated_at)).cloned().collect(),
        deleted: state
            .entries
            .keys()
            .filter(|id| !local_ids.contains(id.as_str()))
            .map(|id| Deletion { id: id.clone(), deleted_at: now_ms })
            .collect(),
        srs: srs
            .iter()
            .filter(|(id, card)| local_ids.contains(id.as_str()) && state.srs.get(*id) != Some(&reviewed_at(card)))
            .map(|(id, card)| (id.clone(), card.clone()))
            .collect(),
    }
}

/// Merges a response into the local data and returns the new sync state.
///
/// Local changes made while the request was in flight are kept: a server
/// version only replaces a local one that is not newer, and anything left
/// different from the returned state is sent again next time.
pub fn apply(entries: &mut Vec<DailyEntry>, srs: &mut Schedule, state: &SyncState, res: SyncResponse) -> SyncState {
    let mut next = SyncState { cursor: res.cursor, ..state.clone() };

    for server in res.entries {
        next.entries.insert(server.id.clone(), server.updated_at);
        match entries.iter().position(|e| e.id == server.id) {
            Some(i) if entries[i].updated_at > server.updated_at => {} // edited meanwhile
            Some(i) => entries[i] = server,
            // Deleted here meanwhile (we had the server's version or newer): the
            // deletion goes out next time. Otherwise it's new to this device.
            None if state.entries.get(&server.id).is_some_and(|&t| t >= server.updated_at) => {}
            None => model::upsert(entries, server),
        }
    }
    for gone in res.deleted {
        next.entries.remove(&gone.id);
        next.srs.remove(&gone.id);
        let newer_here = entries.iter().any(|e| e.id == gone.id && e.updated_at > gone.deleted_at);
        if !newer_here {
            entries.retain(|e| e.id != gone.id);
            srs.remove(&gone.id);
        }
    }
    for (id, card) in res.srs {
        next.srs.insert(id.clone(), reviewed_at(&card));
        let local_newer = srs.get(&id).is_some_and(|c| reviewed_at(c) > reviewed_at(&card));
        if !local_newer && entries.iter().any(|e| e.id == id) {
            srs.insert(id, card);
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EntryDraft;
    use crate::srs::Rating;

    fn entry(id: &str, updated: i64) -> DailyEntry {
        let draft = EntryDraft { japanese: id.into(), translation: "t".into(), ..Default::default() };
        let mut e = draft.build(None, || id.into(), "2026-11-12".parse().unwrap(), updated).unwrap();
        e.updated_at = updated;
        e
    }

    fn state(entries: &[(&str, i64)]) -> SyncState {
        SyncState { cursor: 7, entries: entries.iter().map(|(id, t)| (id.to_string(), *t)).collect(), srs: BTreeMap::new() }
    }

    #[test]
    fn first_sync_sends_everything() {
        let list = [entry("a", 1), entry("b", 2)];
        let mut srs = Schedule::new();
        srs.insert("a".into(), SrsCard::default().review(Rating::Pass, 5));
        srs.insert("gone".into(), SrsCard::default());
        let req = outgoing(&list, &srs, &SyncState::default(), 100);
        assert_eq!(req.cursor, 0);
        assert_eq!(req.entries.len(), 2);
        assert!(req.deleted.is_empty());
        assert_eq!(req.srs.keys().collect::<Vec<_>>(), ["a"], "cards for missing words are not sent");
    }

    #[test]
    fn later_syncs_send_only_changes_and_deletions() {
        let list = [entry("same", 1), entry("edited", 9), entry("new", 3)];
        let st = state(&[("same", 1), ("edited", 2), ("removed", 4)]);
        let req = outgoing(&list, &Schedule::new(), &st, 100);
        let ids: Vec<&str> = req.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["edited", "new"]);
        assert_eq!(req.deleted, [Deletion { id: "removed".into(), deleted_at: 100 }]);
        assert_eq!(req.cursor, 7);
    }

    #[test]
    fn apply_takes_server_changes() {
        let mut list = vec![entry("a", 1), entry("b", 1)];
        let mut srs = Schedule::new();
        srs.insert("b".into(), SrsCard::default());
        let st = state(&[("a", 1), ("b", 1)]);
        let res = SyncResponse {
            cursor: 20,
            entries: vec![entry("a", 5), entry("from-phone", 6)],
            deleted: vec![Deletion { id: "b".into(), deleted_at: 8 }],
            srs: BTreeMap::from([("a".to_string(), SrsCard::default().review(Rating::Pass, 9))]),
        };
        let next = apply(&mut list, &mut srs, &st, res);
        let ids: Vec<(&str, i64)> = list.iter().map(|e| (e.id.as_str(), e.updated_at)).collect();
        assert_eq!(ids, [("a", 5), ("from-phone", 6)]);
        assert_eq!(srs.keys().collect::<Vec<_>>(), ["a"]);
        assert_eq!(next.cursor, 20);
        assert_eq!(next.entries, BTreeMap::from([("a".to_string(), 5), ("from-phone".to_string(), 6)]));
        assert_eq!(next.srs, BTreeMap::from([("a".to_string(), 9)]));
        // Nothing left to send.
        let again = outgoing(&list, &srs, &next, 100);
        assert!(again.entries.is_empty() && again.deleted.is_empty() && again.srs.is_empty());
    }

    #[test]
    fn edits_and_deletions_made_during_a_sync_are_kept() {
        // Sent "a"@2 and "b"@2; meanwhile "a" was edited (@3) and "b" deleted.
        let mut list = vec![entry("a", 3)];
        let mut srs = Schedule::new();
        let st = state(&[("a", 1), ("b", 2)]);
        let res = SyncResponse { cursor: 9, entries: vec![entry("a", 2), entry("b", 2)], ..Default::default() };
        let next = apply(&mut list, &mut srs, &st, res);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].updated_at, 3, "local edit wins");
        let again = outgoing(&list, &srs, &next, 50);
        assert_eq!(again.entries.len(), 1, "the edit goes out next time");
        assert_eq!(again.deleted, [Deletion { id: "b".into(), deleted_at: 50 }], "so does the deletion");
    }

    #[test]
    fn newer_local_word_survives_a_remote_deletion() {
        let mut list = vec![entry("a", 10)];
        let mut srs = Schedule::new();
        let st = state(&[("a", 1)]);
        let res = SyncResponse { cursor: 3, deleted: vec![Deletion { id: "a".into(), deleted_at: 5 }], ..Default::default() };
        let next = apply(&mut list, &mut srs, &st, res);
        assert_eq!(list.len(), 1);
        assert_eq!(outgoing(&list, &srs, &next, 0).entries.len(), 1, "re-sent, so the server brings it back");
    }

    #[test]
    fn wire_format_is_camel_case() {
        let req = SyncRequest { deleted: vec![Deletion { id: "x".into(), deleted_at: 1 }], ..Default::default() };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["deleted"][0]["deletedAt"], 1);
        let back: SyncResponse = serde_json::from_str(r#"{"cursor":4}"#).unwrap();
        assert_eq!(back.cursor, 4);
    }
}
