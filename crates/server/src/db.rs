//! Rows, conversions, and the writes shared by several routes.
//!
//! Writes follow "newest wins": a word only replaces the stored one if its
//! `updatedAt` is not older, and an SM-2 card only if its `lastReviewedAt`
//! is not older. Every write bumps `change_seq` so sync can find it.

use sabi_core::model::{DailyEntry, ExampleSentence};
use sabi_core::srs::SrsCard;
use sabi_core::sync::Deletion;
use serde::Serialize;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{AppError, ApiResult};

#[derive(FromRow)]
pub struct EntryRow {
    pub id: String,
    pub date: String,
    pub japanese: String,
    pub reading: String,
    pub romaji: Option<String>,
    pub translation: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub is_favorite: bool,
    pub example_japanese: Option<String>,
    pub example_reading: Option<String>,
    pub example_translation: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted: bool,
}

pub const ENTRY_COLUMNS: &str = "id, date, japanese, reading, romaji, translation, notes, tags, is_favorite, \
     example_japanese, example_reading, example_translation, created_at, updated_at, deleted";

impl EntryRow {
    pub fn into_entry(self) -> ApiResult<DailyEntry> {
        let date = self.date.parse().map_err(|_| {
            tracing::error!("bad date {:?} stored for entry {}", self.date, self.id);
            AppError::internal()
        })?;
        let example_sentence = self.example_japanese.map(|japanese| ExampleSentence {
            japanese,
            reading: self.example_reading,
            translation: self.example_translation.unwrap_or_default(),
        });
        Ok(DailyEntry {
            id: self.id,
            date,
            japanese: self.japanese,
            reading: self.reading,
            romaji: self.romaji,
            translation: self.translation,
            notes: self.notes,
            tags: self.tags,
            is_favorite: self.is_favorite,
            example_sentence,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }

    pub fn tombstone(&self) -> Deletion {
        Deletion { id: self.id.clone(), deleted_at: self.updated_at }
    }
}

/// Live (not deleted) entries.
pub fn live(rows: Vec<EntryRow>) -> ApiResult<Vec<DailyEntry>> {
    rows.into_iter().filter(|r| !r.deleted).map(EntryRow::into_entry).collect()
}

pub async fn find_entry(conn: &mut PgConnection, user: Uuid, id: &str) -> ApiResult<Option<EntryRow>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM daily_entries WHERE user_id = $1 AND id = $2");
    Ok(sqlx::query_as::<_, EntryRow>(&sql).bind(user).bind(id).fetch_optional(conn).await?)
}

pub async fn all_entries(conn: &mut PgConnection, user: Uuid) -> ApiResult<Vec<DailyEntry>> {
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM daily_entries WHERE user_id = $1 AND NOT deleted");
    live(sqlx::query_as::<_, EntryRow>(&sql).bind(user).fetch_all(conn).await?)
}

/// Inserts or replaces a word unless the stored one is newer. Returns whether
/// it was written. A deleted word comes back if this version is newer.
pub async fn upsert_entry(conn: &mut PgConnection, user: Uuid, e: &DailyEntry) -> ApiResult<bool> {
    let ex = e.example_sentence.as_ref();
    let written = sqlx::query(
        "INSERT INTO daily_entries (user_id, id, date, japanese, reading, romaji, translation, notes, tags, is_favorite,
             example_japanese, example_reading, example_translation, created_at, updated_at, deleted)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, false)
         ON CONFLICT (user_id, id) DO UPDATE SET
             date = EXCLUDED.date, japanese = EXCLUDED.japanese, reading = EXCLUDED.reading,
             romaji = EXCLUDED.romaji, translation = EXCLUDED.translation, notes = EXCLUDED.notes,
             tags = EXCLUDED.tags, is_favorite = EXCLUDED.is_favorite,
             example_japanese = EXCLUDED.example_japanese, example_reading = EXCLUDED.example_reading,
             example_translation = EXCLUDED.example_translation, created_at = EXCLUDED.created_at,
             updated_at = EXCLUDED.updated_at, deleted = false, change_seq = nextval('change_seq')
         WHERE daily_entries.updated_at <= EXCLUDED.updated_at",
    )
    .bind(user)
    .bind(&e.id)
    .bind(e.date.to_string())
    .bind(&e.japanese)
    .bind(&e.reading)
    .bind(&e.romaji)
    .bind(&e.translation)
    .bind(&e.notes)
    .bind(&e.tags)
    .bind(e.is_favorite)
    .bind(ex.map(|x| &x.japanese))
    .bind(ex.and_then(|x| x.reading.as_ref()))
    .bind(ex.map(|x| &x.translation))
    .bind(e.created_at)
    .bind(e.updated_at)
    .execute(&mut *conn)
    .await?
    .rows_affected()
        > 0;
    Ok(written)
}

/// Marks a word deleted at `deleted_at` unless it changed after that. Its SM-2
/// card goes too. Returns whether it was deleted.
pub async fn delete_entry(conn: &mut PgConnection, user: Uuid, id: &str, deleted_at: i64) -> ApiResult<bool> {
    let deleted = sqlx::query(
        "UPDATE daily_entries SET deleted = true, updated_at = $3, change_seq = nextval('change_seq')
         WHERE user_id = $1 AND id = $2 AND NOT deleted AND updated_at <= $3",
    )
    .bind(user)
    .bind(id)
    .bind(deleted_at)
    .execute(&mut *conn)
    .await?
    .rows_affected()
        > 0;
    if deleted {
        sqlx::query("DELETE FROM card_reviews WHERE user_id = $1 AND entry_id = $2").bind(user).bind(id).execute(conn).await?;
    }
    Ok(deleted)
}

#[derive(FromRow)]
pub struct CardRow {
    pub entry_id: String,
    pub interval: i32,
    pub ease: f64,
    pub repetitions: i32,
    pub lapses: i32,
    pub due_at: i64,
    pub last_reviewed_at: Option<i64>,
}

pub const CARD_COLUMNS: &str = "entry_id, interval, ease, repetitions, lapses, due_at, last_reviewed_at";

impl CardRow {
    pub fn into_card(self) -> (String, SrsCard) {
        let n = |v: i32| u32::try_from(v).unwrap_or(0);
        let card = SrsCard {
            interval: n(self.interval),
            ease: self.ease,
            repetitions: n(self.repetitions),
            lapses: n(self.lapses),
            due_at: self.due_at,
            last_reviewed_at: self.last_reviewed_at,
        };
        (self.entry_id, card)
    }
}

/// Saves a card if its word exists and the stored card isn't newer.
pub async fn upsert_card(conn: &mut PgConnection, user: Uuid, entry_id: &str, c: &SrsCard) -> ApiResult<bool> {
    let n = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    let written = sqlx::query(
        "INSERT INTO card_reviews (user_id, entry_id, interval, ease, repetitions, lapses, due_at, last_reviewed_at)
         SELECT $1, $2, $3, $4, $5, $6, $7, $8
         WHERE EXISTS (SELECT 1 FROM daily_entries WHERE user_id = $1 AND id = $2 AND NOT deleted)
         ON CONFLICT (user_id, entry_id) DO UPDATE SET
             interval = EXCLUDED.interval, ease = EXCLUDED.ease, repetitions = EXCLUDED.repetitions,
             lapses = EXCLUDED.lapses, due_at = EXCLUDED.due_at, last_reviewed_at = EXCLUDED.last_reviewed_at,
             change_seq = nextval('change_seq')
         WHERE COALESCE(card_reviews.last_reviewed_at, 0) <= COALESCE(EXCLUDED.last_reviewed_at, 0)",
    )
    .bind(user)
    .bind(entry_id)
    .bind(n(c.interval))
    .bind(c.ease)
    .bind(n(c.repetitions))
    .bind(n(c.lapses))
    .bind(c.due_at)
    .bind(c.last_reviewed_at)
    .execute(conn)
    .await?
    .rows_affected()
        > 0;
    Ok(written)
}

pub async fn all_cards(conn: &mut PgConnection, user: Uuid) -> ApiResult<sabi_core::srs::Schedule> {
    let sql = format!("SELECT {CARD_COLUMNS} FROM card_reviews WHERE user_id = $1");
    let rows = sqlx::query_as::<_, CardRow>(&sql).bind(user).fetch_all(conn).await?;
    Ok(rows.into_iter().map(CardRow::into_card).collect())
}

/// Highest change number for this user (the sync cursor).
pub async fn cursor(conn: &mut PgConnection, user: Uuid) -> ApiResult<i64> {
    let (seq,): (i64,) = sqlx::query_as(
        "SELECT GREATEST(
             COALESCE((SELECT MAX(change_seq) FROM daily_entries WHERE user_id = $1), 0),
             COALESCE((SELECT MAX(change_seq) FROM card_reviews WHERE user_id = $1), 0))",
    )
    .bind(user)
    .fetch_one(conn)
    .await?;
    Ok(seq)
}

#[derive(FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    #[serde(rename = "dailyTargetLimit")]
    pub daily_limit: i32,
}

pub const USER_COLUMNS: &str = "id, email, name, daily_limit";

pub async fn find_user(conn: &mut PgConnection, id: Uuid) -> ApiResult<User> {
    let sql = format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1");
    sqlx::query_as::<_, User>(&sql)
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| AppError::not_found("User not found"))
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// Checks and tidies a word sent by a client, using the backup-import rules
/// from `sabi-core` (id, Japanese text and a valid date are required).
pub fn clean_entry(value: serde_json::Value, now: i64) -> ApiResult<DailyEntry> {
    const MAX_TEXT: usize = 20_000;
    let mut got = sabi_core::import::normalize(std::slice::from_ref(&value), now);
    let entry = got
        .entries
        .pop()
        .ok_or_else(|| AppError::bad_request("A word needs an id, Japanese text and a date (YYYY-MM-DD)"))?;
    let text = [&entry.japanese, &entry.reading, &entry.translation].iter().map(|s| s.len()).sum::<usize>()
        + entry.notes.as_ref().map_or(0, String::len)
        + entry.romaji.as_ref().map_or(0, String::len);
    if entry.id.len() > 128 || text > MAX_TEXT || entry.tags.len() > 50 {
        return Err(AppError::bad_request("This word is too large"));
    }
    Ok(entry)
}
