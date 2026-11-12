//! Streaks and counts, computed with the same code as the web app.

use axum::extract::State;
use axum::Json;
use sabi_core::{DateKey, StreakStats};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::db;
use crate::error::{AppError, ApiResult, Params};
use crate::AppState;

#[derive(Deserialize)]
pub struct StatsQuery {
    /// The client's local date. Defaults to today in UTC.
    today: Option<String>,
}

/// Today's date in UTC, for callers that don't send their own.
pub fn utc_today() -> DateKey {
    let days = db::now_ms().div_euclid(86_400_000);
    let mut d = DateKey::new(1970, 1, 1).expect("valid date");
    // Jump by years first so this stays cheap.
    let mut left = days;
    while left >= 366 {
        let next_year = DateKey::new(d.year() + 1, d.month(), d.day()).expect("Jan 1 exists");
        let len = if sabi_core::date::is_leap_year(d.year()) { 366 } else { 365 };
        d = next_year;
        left -= len;
    }
    for _ in 0..left {
        d = d.next();
    }
    d
}

pub async fn get(State(state): State<AppState>, AuthUser(user): AuthUser, Params(q): Params<StatsQuery>) -> ApiResult<Json<Value>> {
    let today = match q.today {
        Some(t) => t.parse().map_err(|_| AppError::bad_request("today must be YYYY-MM-DD"))?,
        None => utc_today(),
    };
    let mut conn = state.pool.acquire().await?;
    let profile = db::find_user(&mut conn, user).await?;
    let entries = db::all_entries(&mut conn, user).await?;
    let s = StreakStats::compute(&entries, today);
    Ok(Json(json!({
        "stats": {
            "totalEntries": s.total_entries,
            "favoriteCount": s.favorites,
            "entriesToday": s.today_count,
            "dailyTargetLimit": profile.daily_limit,
            "currentStreak": s.current,
            "longestStreak": s.longest,
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_today_is_a_recent_date() {
        let t = utc_today();
        assert!(t.year() >= 2024, "{t}");
    }
}
