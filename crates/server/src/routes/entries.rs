//! Words: list, read, create, edit, delete.
//!
//! Unlike the Express API, the daily goal never blocks a new word, and the
//! client may choose the id (the web app creates words offline).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::db::{self, ENTRY_COLUMNS, EntryRow, now_ms};
use crate::error::{AppError, ApiResult, Body, Params};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListQuery {
    date: Option<String>,
    favorite: Option<bool>,
    tag: Option<String>,
    /// A sync cursor: only words changed after it, including deletions.
    since: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Params(q): Params<ListQuery>,
) -> ApiResult<Json<Value>> {
    let mut conn = state.pool.acquire().await?;
    let sql = format!(
        "SELECT {ENTRY_COLUMNS} FROM daily_entries
         WHERE user_id = $1
           AND ($2::text IS NULL OR date = $2)
           AND ($3::bool IS NULL OR is_favorite = $3)
           AND ($4::text IS NULL OR $4 = ANY(tags))
           AND (CASE WHEN $5::bigint IS NULL THEN NOT deleted ELSE change_seq > $5 END)
         ORDER BY created_at DESC"
    );
    let rows = sqlx::query_as::<_, EntryRow>(&sql)
        .bind(user)
        .bind(q.date)
        .bind(q.favorite)
        .bind(q.tag)
        .bind(q.since)
        .fetch_all(&mut *conn)
        .await?;

    if q.since.is_none() {
        return Ok(Json(json!({ "entries": db::live(rows)? })));
    }
    let deleted: Vec<_> = rows.iter().filter(|r| r.deleted).map(EntryRow::tombstone).collect();
    let cursor = db::cursor(&mut conn, user).await?;
    Ok(Json(json!({ "entries": db::live(rows)?, "deleted": deleted, "cursor": cursor })))
}

pub async fn get_one(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let row = db::find_entry(&mut *state.pool.acquire().await?, user, &id).await?;
    let entry = row.filter(|r| !r.deleted).ok_or_else(|| AppError::not_found("Entry not found"))?.into_entry()?;
    Ok(Json(json!({ "entry": entry })))
}

pub async fn create(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Body(mut value): Body<Value>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let obj = value.as_object_mut().ok_or_else(|| AppError::bad_request("Expected a word object"))?;
    if !obj.get("id").is_some_and(|v| v.as_str().is_some_and(|s| !s.trim().is_empty())) {
        obj.insert("id".into(), Value::String(uuid::Uuid::new_v4().to_string()));
    }
    let now = now_ms();
    let mut entry = db::clean_entry(value, now)?;
    entry.updated_at = entry.updated_at.max(now);

    let mut tx = state.pool.begin().await?;
    if db::find_entry(&mut tx, user, &entry.id).await?.is_some_and(|r| !r.deleted) {
        return Err(AppError::conflict("A word with this id already exists"));
    }
    db::upsert_entry(&mut tx, user, &entry).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(json!({ "entry": entry }))))
}

/// Partial update: send only the fields to change.
pub async fn update(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Path(id): Path<String>,
    Body(patch): Body<Value>,
) -> ApiResult<Json<Value>> {
    let patch = patch.as_object().ok_or_else(|| AppError::bad_request("Expected a word object"))?.clone();
    let mut tx = state.pool.begin().await?;
    let existing = db::find_entry(&mut tx, user, &id)
        .await?
        .filter(|r| !r.deleted)
        .ok_or_else(|| AppError::not_found("Entry not found"))?
        .into_entry()?;

    let mut merged = serde_json::to_value(&existing).map_err(|_| AppError::internal())?;
    let obj = merged.as_object_mut().ok_or_else(AppError::internal)?;
    for (key, val) in patch {
        if !matches!(key.as_str(), "id" | "createdAt" | "updatedAt") {
            obj.insert(key, val);
        }
    }
    let now = now_ms();
    let mut entry = db::clean_entry(merged, now)?;
    entry.updated_at = now.max(existing.updated_at + 1);
    db::upsert_entry(&mut tx, user, &entry).await?;
    tx.commit().await?;
    Ok(Json(json!({ "entry": entry })))
}

pub async fn remove(State(state): State<AppState>, AuthUser(user): AuthUser, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let existing = db::find_entry(&mut tx, user, &id).await?.filter(|r| !r.deleted);
    let existing = existing.ok_or_else(|| AppError::not_found("Entry not found"))?;
    db::delete_entry(&mut tx, user, &id, now_ms().max(existing.updated_at + 1)).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
