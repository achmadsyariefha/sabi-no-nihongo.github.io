//! Settings from environment variables.

use axum::http::HeaderValue;

/// The GitHub Pages site and the local dev server.
const DEFAULT_ORIGINS: &str = "https://achmadsyariefha.github.io,http://localhost:8080";

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub allowed_origins: Vec<HeaderValue>,
}

impl Config {
    /// Reads `DATABASE_URL`, `JWT_SECRET` (at least 32 characters), `PORT`
    /// (default 5000) and `ALLOWED_ORIGINS` (comma-separated).
    pub fn from_env() -> Result<Self, String> {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.trim().is_empty());

        let database_url = var("DATABASE_URL").ok_or("DATABASE_URL is required")?;
        let jwt_secret = var("JWT_SECRET").ok_or("JWT_SECRET is required")?;
        if jwt_secret.len() < 32 {
            return Err("JWT_SECRET must be at least 32 characters".into());
        }
        let port = match var("PORT") {
            Some(p) => p.parse().map_err(|_| format!("PORT is not a number: {p}"))?,
            None => 5000,
        };
        let origins = var("ALLOWED_ORIGINS").unwrap_or_else(|| DEFAULT_ORIGINS.into());
        let allowed_origins = parse_origins(&origins)?;
        Ok(Self { database_url, jwt_secret, port, allowed_origins })
    }
}

pub fn parse_origins(list: &str) -> Result<Vec<HeaderValue>, String> {
    list.split(',')
        .map(|o| o.trim().trim_end_matches('/'))
        .filter(|o| !o.is_empty())
        .map(|o| HeaderValue::from_str(o).map_err(|_| format!("bad origin in ALLOWED_ORIGINS: {o}")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origins_are_trimmed() {
        let got = parse_origins(" https://a.example/ , http://localhost:8080,").unwrap();
        assert_eq!(got, ["https://a.example", "http://localhost:8080"]);
    }
}
