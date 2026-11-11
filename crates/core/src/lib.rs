//! Core logic for Sabi no Nihongo.
//!
//! This crate never touches the browser. Anything that needs `window`,
//! storage, or the current time lives in `sabi-web` and passes values in.

pub mod date;
pub mod proverbs;
pub mod settings;

pub use date::DateKey;
