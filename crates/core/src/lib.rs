//! Core logic for Sabi no Nihongo.
//!
//! This crate never touches the browser. Anything that needs `window`,
//! storage, or the current time lives in `sabi-web` and passes values in.

pub mod date;
pub mod model;
pub mod proverbs;
pub mod settings;
pub mod streak;
pub mod tags;
pub mod vault;

pub use date::DateKey;
pub use model::{DailyEntry, DailyLimit, EntryDraft};
pub use streak::StreakStats;
