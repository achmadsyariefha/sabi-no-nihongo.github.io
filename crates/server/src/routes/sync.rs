//! Two-way sync for the web app (see `sabi_core::sync`).
//!
//! Applies the client's changes (newest wins), then returns everything that
//! changed since the client's cursor plus the current server copy of every
//! word and card the client sent, so the client always ends up agreeing.

use std::collections::BTreeMap;

use axum::extract::State;
use axum::Json;
use sabi_core::sync::{SyncRequest, SyncResponse};
use serde_json::Value;

use crate::auth::AuthUser;
use crate::db::{self, CARD_COLUMNS, CardRow, ENTRY_COLUMNS, EntryRow, now_ms};
use crate::error::{AppError, ApiResult, Body};
use crate::AppState;

const MAX_ITEMS: usize = 10_000;

/// The request with each word checked by the import rules, so one bad word
/// can't be stored.
struct Incoming {
    req: SyncRequest,
}

fn parse(raw: Value) -> ApiResult<Incoming> {
    let mut raw = raw;
    let entries = raw.as_object_mut().and_then(|o| o.remove("entries")).unwrap_or(Value::Array(Vec::new()));
    let mut req: SyncRequest = serde_json::from_value(raw).map_err(|e| AppError::bad_request(format!("Invalid sync request: {e}")))?;
    let Value::Array(items) = entries else {
        return Err(AppError::bad_request("entries must be a list"));
    };
    if items.len() > MAX_ITEMS || req.deleted.len() > MAX_ITEMS || req.srs.len() > MAX_ITEMS {
        return Err(AppError::bad_request("Too many changes in one sync"));
    }
    let now = now_ms();
    req.entries = items.into_iter().map(|v| db::clean_entry(v, now)).collect::<ApiResult<_>>()?;
    Ok(Incoming { req })
}

pub async fn sync(State(state): State<AppState>, AuthUser(user): AuthUser, Body(raw): Body<Value>) -> ApiResult<Json<SyncResponse>> {
    let Incoming { req } = parse(raw)?;
    let mut tx = state.pool.begin().await?;

    for e in &req.entries {
        db::upsert_entry(&mut tx, user, e).await?;
    }
    for d in &req.deleted {
        db::delete_entry(&mut tx, user, &d.id, d.deleted_at).await?;
    }
    for (id, card) in &req.srs {
        db::upsert_card(&mut tx, user, id, card).await?;
    }

    let touched: Vec<String> = req.entries.iter().map(|e| e.id.clone()).chain(req.deleted.iter().map(|d| d.id.clone())).collect();
    let sql = format!("SELECT {ENTRY_COLUMNS} FROM daily_entries WHERE user_id = $1 AND (change_seq > $2 OR id = ANY($3))");
    let rows = sqlx::query_as::<_, EntryRow>(&sql).bind(user).bind(req.cursor).bind(&touched).fetch_all(&mut *tx).await?;
    let deleted = rows.iter().filter(|r| r.deleted).map(EntryRow::tombstone).collect();
    let entries = db::live(rows)?;

    let sent_cards: Vec<String> = req.srs.keys().cloned().collect();
    let sql = format!("SELECT {CARD_COLUMNS} FROM card_reviews WHERE user_id = $1 AND (change_seq > $2 OR entry_id = ANY($3))");
    let cards = sqlx::query_as::<_, CardRow>(&sql).bind(user).bind(req.cursor).bind(&sent_cards).fetch_all(&mut *tx).await?;
    let srs: BTreeMap<_, _> = cards.into_iter().map(CardRow::into_card).collect();

    let cursor = db::cursor(&mut tx, user).await?;
    tx.commit().await?;
    Ok(Json(SyncResponse { cursor, entries, deleted, srs }))
}
