//! Typed, versioned access to `localStorage`.
//!
//! Every value is stored as `{ "v": 1, "data": ... }` so later releases can
//! migrate old shapes. Read errors never panic: they fall back to defaults.

use std::collections::BTreeMap;

use gloo_storage::{LocalStorage, Storage};
use sabi_core::activity::{self, DayCounts};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Versioned<T> {
    v: u32,
    data: T,
}

pub fn load<T: DeserializeOwned + Default>(key: &str) -> T {
    match LocalStorage::get::<Versioned<T>>(key) {
        Ok(stored) if stored.v == VERSION => stored.data,
        Ok(_) | Err(_) => T::default(),
    }
}

/// Loads a list item by item, skipping items that no longer parse.
///
/// If anything is skipped (or the whole value is unreadable), the raw stored
/// text is copied to `<key>.backup` first, so the next save can't silently
/// destroy data.
pub fn load_list<T: DeserializeOwned>(key: &str) -> Vec<T> {
    let Ok(Some(raw)) = LocalStorage::raw().get_item(key) else {
        return Vec::new();
    };
    let items = serde_json::from_str::<Versioned<Vec<serde_json::Value>>>(&raw)
        .ok()
        .filter(|stored| stored.v == VERSION)
        .map(|stored| stored.data);
    let Some(items) = items else {
        keep_backup(key, &raw, "unreadable");
        return Vec::new();
    };
    let total = items.len();
    let list: Vec<T> = items.into_iter().filter_map(|item| serde_json::from_value(item).ok()).collect();
    if list.len() < total {
        keep_backup(key, &raw, &format!("skipped {} of {total} items", total - list.len()));
    }
    list
}

/// Loads a `{ "YYYY-MM-DD": count }` map, dropping bad pairs (with a backup,
/// as in [`load_list`]).
pub fn load_day_counts(key: &str) -> DayCounts {
    let Ok(Some(raw)) = LocalStorage::raw().get_item(key) else {
        return DayCounts::new();
    };
    let map = serde_json::from_str::<Versioned<BTreeMap<String, serde_json::Value>>>(&raw)
        .ok()
        .filter(|stored| stored.v == VERSION)
        .map(|stored| stored.data);
    let Some(map) = map else {
        keep_backup(key, &raw, "unreadable");
        return DayCounts::new();
    };
    let (counts, dropped) = activity::review_log_from_pairs(map.iter().map(|(k, v)| (k.as_str(), v.as_u64())));
    if dropped > 0 {
        keep_backup(key, &raw, &format!("dropped {dropped} bad days"));
    }
    counts
}

fn keep_backup(key: &str, raw: &str, reason: &str) {
    let backup_key = format!("{key}.backup");
    leptos::logging::warn!("{key}: {reason}; raw data kept in {backup_key}");
    let _ = LocalStorage::raw().set_item(&backup_key, raw);
}

pub fn save<T: Serialize>(key: &str, value: &T) {
    let wrapped = Versioned { v: VERSION, data: value };
    if let Err(err) = LocalStorage::set(key, wrapped) {
        leptos::logging::warn!("could not save {key}: {err}");
    }
}

pub fn remove(key: &str) {
    LocalStorage::delete(key);
}
