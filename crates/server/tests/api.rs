//! End-to-end tests against a real Postgres.
//!
//! Set `SABI_TEST_DATABASE_URL` to an empty database you don't mind tests
//! writing to (each test makes its own users). Without it these tests skip.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use sabi_core::DailyEntry;
use sabi_core::srs::Schedule;
use sabi_core::sync::{self, SyncResponse, SyncState};
use serde_json::{Value, json};
use tower::ServiceExt;

const SECRET: &str = "test-secret-that-is-at-least-32-chars";

async fn app() -> Option<Router> {
    let Ok(url) = std::env::var("SABI_TEST_DATABASE_URL") else {
        eprintln!("SABI_TEST_DATABASE_URL not set; skipping");
        return None;
    };
    let pool = sqlx::postgres::PgPoolOptions::new().max_connections(5).connect(&url).await.expect("connect");
    sabi_server::migrate(&pool).await.expect("migrate");
    let origins = sabi_server::config::parse_origins("http://localhost:8080").unwrap();
    Some(sabi_server::app(sabi_server::AppState::new(pool, SECRET), origins))
}

async fn call(app: &Router, method: &str, path: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req.header("content-type", "application/json").body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
    (status, value)
}

fn unique_email() -> String {
    format!("{}@test.example", uuid::Uuid::new_v4())
}

async fn new_user(app: &Router) -> String {
    let (status, body) =
        call(app, "POST", "/api/auth/register", None, Some(json!({ "email": unique_email(), "password": "password123" }))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["token"].as_str().unwrap().to_owned()
}

fn word(id: &str, date: &str, japanese: &str, updated: i64) -> Value {
    json!({ "id": id, "date": date, "japanese": japanese, "reading": "", "translation": "t",
            "tags": ["Favorite"], "isFavorite": true, "createdAt": updated, "updatedAt": updated })
}

#[tokio::test]
async fn accounts() {
    let Some(app) = app().await else { return };
    let email = unique_email();
    let creds = json!({ "email": email, "password": "password123", "name": "Ana" });

    let (status, body) = call(&app, "POST", "/api/auth/register", None, Some(creds.clone())).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["user"]["email"], email);
    assert_eq!(body["user"]["dailyTargetLimit"], 1);

    let (status, _) = call(&app, "POST", "/api/auth/register", None, Some(creds.clone())).await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (status, body) = call(&app, "POST", "/api/auth/register", None, Some(json!({ "email": "x@y.z", "password": "short" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("8 characters"));

    let wrong = json!({ "email": email, "password": "wrong-password" });
    assert_eq!(call(&app, "POST", "/api/auth/login", None, Some(wrong)).await.0, StatusCode::UNAUTHORIZED);
    let nobody = json!({ "email": unique_email(), "password": "password123" });
    assert_eq!(call(&app, "POST", "/api/auth/login", None, Some(nobody)).await.0, StatusCode::UNAUTHORIZED);

    let (status, body) =
        call(&app, "POST", "/api/auth/login", None, Some(json!({ "email": email.to_uppercase(), "password": "password123" }))).await;
    assert_eq!(status, StatusCode::OK);
    let token = body["token"].as_str().unwrap();

    let (status, me) = call(&app, "GET", "/api/auth/me", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["name"], "Ana");

    let (status, me) = call(&app, "PATCH", "/api/auth/me", Some(token), Some(json!({ "dailyTargetLimit": 3 }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!((me["dailyTargetLimit"].as_i64(), me["name"].as_str()), (Some(3), Some("Ana")));
    let bad = json!({ "dailyTargetLimit": 500 });
    assert_eq!(call(&app, "PATCH", "/api/auth/me", Some(token), Some(bad)).await.0, StatusCode::BAD_REQUEST);

    assert_eq!(call(&app, "GET", "/api/auth/me", None, None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "GET", "/api/auth/me", Some("nope"), None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(call(&app, "GET", "/api/health", None, None).await.1["status"], "ok");
    assert_eq!(call(&app, "GET", "/nowhere", None, None).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn entries_crud_and_isolation() {
    let Some(app) = app().await else { return };
    let token = new_user(&app).await;
    let t = Some(token.as_str());

    let (status, body) = call(&app, "POST", "/api/entries", t, Some(word("w1", "2026-11-10", "猫", 1))).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let (status, _) = call(&app, "POST", "/api/entries", t, Some(word("w1", "2026-11-10", "猫", 1))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    // No id: the server makes one. Goal never blocks a second word that day.
    let (status, body) = call(&app, "POST", "/api/entries", t, Some(json!({ "date": "2026-11-10", "japanese": "犬" }))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert!(!body["entry"]["id"].as_str().unwrap().is_empty());
    let (status, _) = call(&app, "POST", "/api/entries", t, Some(json!({ "date": "2026-02-30", "japanese": "x" }))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (_, list) = call(&app, "GET", "/api/entries?date=2026-11-10", t, None).await;
    assert_eq!(list["entries"].as_array().unwrap().len(), 2);

    let (status, body) = call(&app, "PATCH", "/api/entries/w1", t, Some(json!({ "translation": "cat", "tags": ["#Animals"] }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["entry"]["translation"], "cat");
    assert_eq!(body["entry"]["tags"], json!(["Animals"]));
    assert_eq!(body["entry"]["japanese"], "猫", "unchanged fields are kept");

    let (_, since) = call(&app, "GET", "/api/entries?since=0", t, None).await;
    let cursor = since["cursor"].as_i64().unwrap();
    assert_eq!(call(&app, "DELETE", "/api/entries/w1", t, None).await.0, StatusCode::NO_CONTENT);
    assert_eq!(call(&app, "GET", "/api/entries/w1", t, None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(call(&app, "DELETE", "/api/entries/w1", t, None).await.0, StatusCode::NOT_FOUND);
    let (_, changes) = call(&app, "GET", &format!("/api/entries?since={cursor}"), t, None).await;
    assert_eq!(changes["deleted"][0]["id"], "w1");
    assert!(changes["entries"].as_array().unwrap().is_empty());

    // Another user can use the same id and never sees the first user's words.
    let other = new_user(&app).await;
    let (status, _) = call(&app, "POST", "/api/entries", Some(&other), Some(word("w1", "2026-11-10", "別", 1))).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, theirs) = call(&app, "GET", "/api/entries", Some(&other), None).await;
    assert_eq!(theirs["entries"].as_array().unwrap().len(), 1);
    assert_eq!(theirs["entries"][0]["japanese"], "別");
}

/// A browser: its own words and SM-2 cards plus what it knows of the server.
struct Device {
    entries: Vec<DailyEntry>,
    srs: Schedule,
    state: SyncState,
}

impl Device {
    fn new() -> Self {
        Self { entries: Vec::new(), srs: Schedule::new(), state: SyncState::default() }
    }

    fn add(&mut self, value: Value) {
        self.entries.push(serde_json::from_value(value).unwrap());
    }

    async fn sync(&mut self, app: &Router, token: &str, now: i64) {
        let req = sync::outgoing(&self.entries, &self.srs, &self.state, now);
        let (status, body) = call(app, "POST", "/api/sync", Some(token), Some(serde_json::to_value(&req).unwrap())).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let res: SyncResponse = serde_json::from_value(body).unwrap();
        self.state = sync::apply(&mut self.entries, &mut self.srs, &self.state, res);
    }

    fn words(&self) -> Vec<(String, String)> {
        let mut w: Vec<_> = self.entries.iter().map(|e| (e.id.clone(), e.japanese.clone())).collect();
        w.sort();
        w
    }
}

#[tokio::test]
async fn two_devices_stay_in_sync() {
    let Some(app) = app().await else { return };
    let token = new_user(&app).await;

    let mut laptop = Device::new();
    laptop.add(word("a", "2026-11-10", "雨", 10));
    laptop.add(word("b", "2026-11-11", "雪", 10));
    laptop.srs.insert("a".into(), sabi_core::srs::SrsCard::default().review(sabi_core::srs::Rating::Pass, 15));
    laptop.sync(&app, &token, 20).await;

    let mut phone = Device::new();
    phone.add(word("c", "2026-11-12", "風", 30));
    phone.sync(&app, &token, 40).await;
    assert_eq!(phone.words().len(), 3, "phone gets the laptop's words");
    assert_eq!(phone.srs["a"].interval, 1, "and its SM-2 card");

    // Phone edits "a" and deletes "b"; laptop edits "b" earlier than that.
    phone.entries.iter_mut().find(|e| e.id == "a").unwrap().japanese = "大雨".into();
    phone.entries.iter_mut().find(|e| e.id == "a").unwrap().updated_at = 50;
    phone.entries.retain(|e| e.id != "b");
    laptop.entries.iter_mut().find(|e| e.id == "b").unwrap().updated_at = 45;
    phone.sync(&app, &token, 60).await;
    laptop.sync(&app, &token, 70).await;

    let expected = vec![("a".to_string(), "大雨".to_string()), ("c".to_string(), "風".to_string())];
    assert_eq!(laptop.words(), expected, "newer edit and deletion win");
    assert!(!laptop.srs.contains_key("b"));

    // Laptop edits "c" after the phone deleted it? Newest wins: it comes back.
    phone.entries.retain(|e| e.id != "c");
    phone.sync(&app, &token, 80).await;
    laptop.entries.iter_mut().find(|e| e.id == "c").unwrap().updated_at = 90;
    laptop.sync(&app, &token, 95).await;
    phone.sync(&app, &token, 100).await;
    assert_eq!(phone.words(), laptop.words());
    assert_eq!(phone.words().len(), 2);

    // A sync with nothing new sends and changes nothing.
    let before = laptop.words();
    let idle = sync::outgoing(&laptop.entries, &laptop.srs, &laptop.state, 200);
    assert!(idle.entries.is_empty() && idle.deleted.is_empty() && idle.srs.is_empty());
    laptop.sync(&app, &token, 200).await;
    assert_eq!(laptop.words(), before);

    // Bad words are refused as a whole.
    let bad = json!({ "cursor": 0, "entries": [{ "id": "x", "date": "nope", "japanese": "x" }] });
    assert_eq!(call(&app, "POST", "/api/sync", Some(&token), Some(bad)).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn srs_stats_and_daily_review() {
    let Some(app) = app().await else { return };
    let token = new_user(&app).await;
    let t = Some(token.as_str());
    for (id, date) in [("x", "2026-11-10"), ("y", "2026-11-11"), ("z", "2026-11-11")] {
        call(&app, "POST", "/api/entries", t, Some(word(id, date, id, 1))).await;
    }

    let (_, due) = call(&app, "GET", "/api/srs/due?limit=2", t, None).await;
    assert_eq!(due["dueCount"], 3);
    assert_eq!(due["cards"].as_array().unwrap().len(), 2);
    let (status, body) = call(&app, "POST", "/api/srs/review", t, Some(json!({ "entryId": "x", "rating": "pass" }))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["card"]["interval"], 1);
    let (_, due) = call(&app, "GET", "/api/srs/due", t, None).await;
    assert_eq!(due["dueCount"], 2);
    let missing = json!({ "entryId": "nope", "rating": "fail" });
    assert_eq!(call(&app, "POST", "/api/srs/review", t, Some(missing)).await.0, StatusCode::NOT_FOUND);
    let bad = json!({ "entryId": "x", "rating": "meh" });
    assert_eq!(call(&app, "POST", "/api/srs/review", t, Some(bad)).await.0, StatusCode::BAD_REQUEST);

    let (_, stats) = call(&app, "GET", "/api/stats?today=2026-11-12", t, None).await;
    assert_eq!(stats["stats"]["totalEntries"], 3);
    assert_eq!(stats["stats"]["currentStreak"], 2, "logged the 10th and 11th; today not yet");
    assert_eq!(stats["stats"]["entriesToday"], 0);

    let (_, s) = call(&app, "GET", "/api/review?dateKey=2026-11-11&filterMode=all", t, None).await;
    assert_eq!(s["cards"].as_array().unwrap().len(), 2);
    let mark = json!({ "dateKey": "2026-11-11", "filterMode": "all", "cardId": "y", "status": "known", "currentIndex": 1 });
    let (status, s) = call(&app, "POST", "/api/review/mark", t, Some(mark)).await;
    assert_eq!(status, StatusCode::OK, "{s}");
    assert_eq!(s["session"]["knownCardIds"], json!(["y"]));
    let mark = json!({ "dateKey": "2026-11-11", "cardId": "y", "status": "need-review" });
    let (_, s) = call(&app, "POST", "/api/review/mark", t, Some(mark)).await;
    assert_eq!((s["session"]["knownCardIds"].clone(), s["session"]["needReviewCardIds"].clone()), (json!([]), json!(["y"])));
    let (_, s) = call(&app, "POST", "/api/review/reset", t, Some(json!({ "dateKey": "2026-11-11" }))).await;
    assert_eq!((s["session"]["needReviewCardIds"].clone(), s["session"]["currentIndex"].clone()), (json!([]), json!(0)));
    let wrong_day = json!({ "dateKey": "2026-11-10", "cardId": "y", "status": "known" });
    assert_eq!(call(&app, "POST", "/api/review/mark", t, Some(wrong_day)).await.0, StatusCode::NOT_FOUND);
}
