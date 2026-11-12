//! Register, log in, and the signed-in user's profile.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::auth::{self, AuthUser};
use crate::db::{self, USER_COLUMNS, User};
use crate::error::{AppError, ApiResult, Body};
use crate::AppState;

#[derive(Deserialize)]
pub struct Credentials {
    email: String,
    password: String,
    name: Option<String>,
}

fn clean_email(email: &str) -> ApiResult<String> {
    let email = email.trim().to_lowercase();
    let valid = email.len() <= 254
        && email.split_once('@').is_some_and(|(user, domain)| !user.is_empty() && domain.contains('.') && !domain.starts_with('.'));
    if valid { Ok(email) } else { Err(AppError::bad_request("Invalid email address")) }
}

fn check_password(password: &str) -> ApiResult<()> {
    match password.chars().count() {
        0..8 => Err(AppError::bad_request("Password must be at least 8 characters long")),
        8..=128 => Ok(()),
        _ => Err(AppError::bad_request("Password must be at most 128 characters long")),
    }
}

fn clean_name(name: Option<String>) -> ApiResult<Option<String>> {
    let name = name.map(|n| n.trim().to_owned()).filter(|n| !n.is_empty());
    if name.as_ref().is_some_and(|n| n.chars().count() > 100) {
        return Err(AppError::bad_request("Name must be at most 100 characters long"));
    }
    Ok(name)
}

pub async fn register(State(state): State<AppState>, Body(input): Body<Credentials>) -> ApiResult<(StatusCode, Json<Value>)> {
    let email = clean_email(&input.email)?;
    check_password(&input.password)?;
    let name = clean_name(input.name)?;
    let hash = auth::hash_password(input.password).await?;

    let sql = format!(
        "INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4)
         ON CONFLICT (email) DO NOTHING RETURNING {USER_COLUMNS}"
    );
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(Uuid::new_v4())
        .bind(&email)
        .bind(hash)
        .bind(name)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::conflict("Email already exists"))?;
    let token = auth::sign_token(&state.keys, user.id)?;
    Ok((StatusCode::CREATED, Json(json!({ "user": user, "token": token }))))
}

pub async fn login(State(state): State<AppState>, Body(input): Body<Credentials>) -> ApiResult<Json<Value>> {
    let email = input.email.trim().to_lowercase();
    let found: Option<(Uuid, String)> = sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;
    let (id, hash) = found.map_or((None, None), |(id, hash)| (Some(id), Some(hash)));
    if !auth::verify_password(input.password, hash).await {
        return Err(AppError::unauthorized("Invalid email or password"));
    }
    let id = id.ok_or_else(AppError::internal)?;
    let user = db::find_user(&mut *state.pool.acquire().await?, id).await?;
    let token = auth::sign_token(&state.keys, user.id)?;
    Ok(Json(json!({ "user": user, "token": token })))
}

pub async fn me(State(state): State<AppState>, AuthUser(id): AuthUser) -> ApiResult<Json<User>> {
    Ok(Json(db::find_user(&mut *state.pool.acquire().await?, id).await?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdate {
    name: Option<String>,
    /// Daily goal; 0 means no goal.
    daily_target_limit: Option<i32>,
}

pub async fn update_me(
    State(state): State<AppState>,
    AuthUser(id): AuthUser,
    Body(input): Body<ProfileUpdate>,
) -> ApiResult<Json<User>> {
    if input.daily_target_limit.is_some_and(|n| !(0..=100).contains(&n)) {
        return Err(AppError::bad_request("dailyTargetLimit must be between 0 and 100"));
    }
    let name = clean_name(input.name.clone())?;
    let sql = format!(
        "UPDATE users SET
             name = CASE WHEN $2 THEN $3 ELSE name END,
             daily_limit = COALESCE($4, daily_limit),
             updated_at = now()
         WHERE id = $1 RETURNING {USER_COLUMNS}"
    );
    let user = sqlx::query_as::<_, User>(&sql)
        .bind(id)
        .bind(input.name.is_some())
        .bind(name)
        .bind(input.daily_target_limit)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("User not found"))?;
    Ok(Json(user))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_email_and_password() {
        assert_eq!(clean_email("  Me@Example.COM ").unwrap(), "me@example.com");
        for bad in ["", "me", "@example.com", "me@example", "me@.com"] {
            assert!(clean_email(bad).is_err(), "{bad}");
        }
        assert!(check_password("short").is_err());
        assert!(check_password("long enough").is_ok());
        assert!(check_password(&"x".repeat(129)).is_err());
    }
}
