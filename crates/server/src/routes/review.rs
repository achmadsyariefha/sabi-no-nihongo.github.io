//! The daily "Known / Need review" session, as in the Express API: the
//! cards are the words logged on `dateKey`.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::auth::AuthUser;
use crate::db::{self, ENTRY_COLUMNS, EntryRow};
use crate::error::{AppError, ApiResult, Body, Params};
use crate::AppState;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum FilterMode {
    #[default]
    All,
    Favorites,
}

impl FilterMode {
    fn as_str(self) -> &'static str {
        match self {
            FilterMode::All => "all",
            FilterMode::Favorites => "favorites",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionKey {
    date_key: String,
    #[serde(default)]
    filter_mode: FilterMode,
}

impl SessionKey {
    fn check(&self) -> ApiResult<()> {
        self.date_key.parse::<sabi_core::DateKey>().map(|_| ()).map_err(|_| AppError::bad_request("dateKey must be YYYY-MM-DD"))
    }
}

#[derive(FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    date_key: String,
    filter_mode: String,
    current_index: i32,
    #[serde(rename = "knownCardIds")]
    known_ids: Vec<String>,
    #[serde(rename = "needReviewCardIds")]
    need_review_ids: Vec<String>,
}

const SESSION_COLUMNS: &str = "date_key, filter_mode, current_index, known_ids, need_review_ids";

async fn get_or_create(conn: &mut PgConnection, user: Uuid, key: &SessionKey) -> ApiResult<Session> {
    let sql = format!(
        "INSERT INTO review_sessions (user_id, date_key, filter_mode) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, date_key, filter_mode) DO UPDATE SET user_id = EXCLUDED.user_id
         RETURNING {SESSION_COLUMNS}"
    );
    Ok(sqlx::query_as::<_, Session>(&sql).bind(user).bind(&key.date_key).bind(key.filter_mode.as_str()).fetch_one(conn).await?)
}

pub async fn get_session(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Params(key): Params<SessionKey>,
) -> ApiResult<Json<Value>> {
    key.check()?;
    let mut conn = state.pool.acquire().await?;
    let session = get_or_create(&mut conn, user, &key).await?;
    let sql = format!(
        "SELECT {ENTRY_COLUMNS} FROM daily_entries
         WHERE user_id = $1 AND date = $2 AND NOT deleted AND ($3 = 'all' OR is_favorite)
         ORDER BY created_at ASC"
    );
    let rows = sqlx::query_as::<_, EntryRow>(&sql)
        .bind(user)
        .bind(&key.date_key)
        .bind(key.filter_mode.as_str())
        .fetch_all(&mut *conn)
        .await?;
    Ok(Json(json!({ "session": session, "cards": db::live(rows)? })))
}

pub async fn reset(State(state): State<AppState>, AuthUser(user): AuthUser, Body(key): Body<SessionKey>) -> ApiResult<Json<Value>> {
    key.check()?;
    let sql = format!(
        "INSERT INTO review_sessions (user_id, date_key, filter_mode) VALUES ($1, $2, $3)
         ON CONFLICT (user_id, date_key, filter_mode) DO UPDATE
             SET known_ids = '{{}}', need_review_ids = '{{}}', current_index = 0, updated_at = now()
         RETURNING {SESSION_COLUMNS}"
    );
    let session = sqlx::query_as::<_, Session>(&sql)
        .bind(user)
        .bind(&key.date_key)
        .bind(key.filter_mode.as_str())
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(json!({ "session": session })))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Known,
    NeedReview,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkInput {
    #[serde(flatten)]
    key: SessionKey,
    card_id: String,
    status: Status,
    current_index: Option<i32>,
}

pub async fn mark(State(state): State<AppState>, AuthUser(user): AuthUser, Body(input): Body<MarkInput>) -> ApiResult<Json<Value>> {
    input.key.check()?;
    if input.current_index.is_some_and(|i| i < 0) {
        return Err(AppError::bad_request("currentIndex must be 0 or more"));
    }
    let mut tx = state.pool.begin().await?;
    let card = db::find_entry(&mut tx, user, &input.card_id).await?;
    if !card.is_some_and(|c| !c.deleted && c.date == input.key.date_key) {
        return Err(AppError::not_found("Card not found"));
    }
    let session = get_or_create(&mut tx, user, &input.key).await?;
    let mut known: Vec<String> = session.known_ids.into_iter().filter(|id| *id != input.card_id).collect();
    let mut need: Vec<String> = session.need_review_ids.into_iter().filter(|id| *id != input.card_id).collect();
    match input.status {
        Status::Known => known.push(input.card_id.clone()),
        Status::NeedReview => need.push(input.card_id.clone()),
    }
    let sql = format!(
        "UPDATE review_sessions
         SET known_ids = $4, need_review_ids = $5, current_index = COALESCE($6, current_index), updated_at = now()
         WHERE user_id = $1 AND date_key = $2 AND filter_mode = $3
         RETURNING {SESSION_COLUMNS}"
    );
    let updated = sqlx::query_as::<_, Session>(&sql)
        .bind(user)
        .bind(&input.key.date_key)
        .bind(input.key.filter_mode.as_str())
        .bind(known)
        .bind(need)
        .bind(input.current_index)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "session": updated })))
}
