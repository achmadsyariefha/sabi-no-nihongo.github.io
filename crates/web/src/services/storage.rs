//! Typed, versioned access to `localStorage`.
//!
//! Every value is stored as `{ "v": 1, "data": ... }` so later releases can
//! migrate old shapes. Read errors never panic: they fall back to defaults.

use gloo_storage::{LocalStorage, Storage};
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

pub fn save<T: Serialize>(key: &str, value: &T) {
    let wrapped = Versioned { v: VERSION, data: value };
    if let Err(err) = LocalStorage::set(key, wrapped) {
        leptos::logging::warn!("could not save {key}: {err}");
    }
}
