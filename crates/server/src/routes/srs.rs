//! SM-2 cards, using `sabi_core::srs` (the same code as the web app).

use axum::extract::State;
use axum::Json;
use sabi_core::srs::{self, Rating};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::db::{self, now_ms};
use crate::error::{AppError, ApiResult, Body, Params};
use crate::AppState;

#[derive(Deserialize)]
pub struct DueQuery {
    limit: Option<usize>,
}

/// Words due now: reviews first, then new words.
pub async fn due(State(state): State<AppState>, AuthUser(user): AuthUser, Params(q): Params<DueQuery>) -> ApiResult<Json<Value>> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let mut conn = state.pool.acquire().await?;
    let entries = db::all_entries(&mut conn, user).await?;
    let schedule = db::all_cards(&mut conn, user).await?;
    let now = now_ms();
    let queue = srs::due_queue(&entries, &schedule, now, now + 1);
    let cards: Vec<Value> = queue
        .iter()
        .take(limit)
        .map(|e| json!({ "entry": e, "card": schedule.get(&e.id).cloned().unwrap_or_default() }))
        .collect();
    Ok(Json(json!({ "cards": cards, "dueCount": queue.len() })))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum RatingInput {
    Pass,
    Fail,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewInput {
    entry_id: String,
    rating: RatingInput,
}

pub async fn review(State(state): State<AppState>, AuthUser(user): AuthUser, Body(input): Body<ReviewInput>) -> ApiResult<Json<Value>> {
    let mut tx = state.pool.begin().await?;
    if !db::find_entry(&mut tx, user, &input.entry_id).await?.is_some_and(|r| !r.deleted) {
        return Err(AppError::not_found("Card not found"));
    }
    let schedule = db::all_cards(&mut tx, user).await?;
    let before = schedule.get(&input.entry_id).cloned().unwrap_or_default();
    let (rating, label) = match input.rating {
        RatingInput::Pass => (Rating::Pass, "pass"),
        RatingInput::Fail => (Rating::Fail, "fail"),
    };
    let now = now_ms();
    let after = before.review(rating, now);
    db::upsert_card(&mut tx, user, &input.entry_id, &after).await?;
    sqlx::query(
        "INSERT INTO review_logs (user_id, entry_id, rating, interval_before, interval_after, reviewed_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(user)
    .bind(&input.entry_id)
    .bind(label)
    .bind(i32::try_from(before.interval).unwrap_or(i32::MAX))
    .bind(i32::try_from(after.interval).unwrap_or(i32::MAX))
    .bind(now)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "card": after })))
}
