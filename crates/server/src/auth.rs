//! Passwords (Argon2) and access tokens (JWT, 7 days).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;
use crate::error::AppError;

const TOKEN_DAYS: u64 = 7;

pub struct Keys {
    encoding: EncodingKey,
    decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &str) -> Self {
        Self { encoding: EncodingKey::from_secret(secret.as_bytes()), decoding: DecodingKey::from_secret(secret.as_bytes()) }
    }
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    exp: u64,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn sign_token(keys: &Keys, user_id: Uuid) -> Result<String, AppError> {
    let claims = Claims { sub: user_id, exp: now_secs() + TOKEN_DAYS * 86_400 };
    encode(&Header::default(), &claims, &keys.encoding).map_err(|err| {
        tracing::error!("could not sign token: {err}");
        AppError::internal()
    })
}

pub fn verify_token(keys: &Keys, token: &str) -> Option<Uuid> {
    decode::<Claims>(token, &keys.decoding, &Validation::default()).ok().map(|data| data.claims.sub)
}

/// Hashes on a blocking thread: Argon2 is slow on purpose.
pub async fn hash_password(password: String) -> Result<String, AppError> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string())
    })
    .await
    .ok()
    .and_then(Result::ok)
    .ok_or_else(AppError::internal)
}

/// Checks a password. With no stored hash it still does the work, so a
/// wrong email takes as long as a wrong password.
pub async fn verify_password(password: String, hash: Option<String>) -> bool {
    tokio::task::spawn_blocking(move || {
        let has_user = hash.is_some();
        let hash = hash.unwrap_or_else(|| {
            Argon2::default()
                .hash_password(b"dummy", &SaltString::generate(&mut OsRng))
                .map(|h| h.to_string())
                .unwrap_or_default()
        });
        let matches = PasswordHash::new(&hash)
            .is_ok_and(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok());
        has_user && matches
    })
    .await
    .unwrap_or(false)
}

/// The signed-in user, from `Authorization: Bearer <token>`.
pub struct AuthUser(pub Uuid);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::unauthorized("Missing or invalid Authorization header"))?;
        verify_token(&state.keys, token).map(AuthUser).ok_or_else(|| AppError::unauthorized("Invalid or expired token"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_round_trip_and_reject_other_secrets() {
        let keys = Keys::new("a-secret-that-is-at-least-32-chars!!");
        let id = Uuid::new_v4();
        let token = sign_token(&keys, id).unwrap();
        assert_eq!(verify_token(&keys, &token), Some(id));
        let other = Keys::new("another-secret-that-is-32-chars-long");
        assert_eq!(verify_token(&other, &token), None);
        assert_eq!(verify_token(&keys, "garbage"), None);
    }

    #[tokio::test]
    async fn passwords_verify() {
        let hash = hash_password("correct horse".into()).await.unwrap();
        assert!(verify_password("correct horse".into(), Some(hash.clone())).await);
        assert!(!verify_password("wrong".into(), Some(hash)).await);
        assert!(!verify_password("anything".into(), None).await);
    }
}
