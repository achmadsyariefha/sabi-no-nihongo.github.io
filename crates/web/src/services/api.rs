//! JSON requests to the optional sync server.

use leptos::prelude::window;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, Request, RequestInit, Response};

#[derive(Clone, Debug, PartialEq)]
pub struct ApiError {
    pub message: String,
    /// HTTP status, or `None` if the server couldn't be reached.
    pub status: Option<u16>,
}

impl ApiError {
    fn network() -> Self {
        Self { message: "Couldn't reach the server. Check the address and your connection.".into(), status: None }
    }

    pub fn is_unauthorized(&self) -> bool {
        self.status == Some(401)
    }
}

fn js_err(_: JsValue) -> ApiError {
    ApiError::network()
}

/// Cleans a server address: trims it, drops a trailing `/`, and adds
/// `https://` when no scheme is given (except for localhost).
pub fn normalize_server(input: &str) -> Option<String> {
    let s = input.trim().trim_end_matches('/');
    if s.is_empty() || s.contains(char::is_whitespace) {
        return None;
    }
    if s.starts_with("https://") || s.starts_with("http://") {
        Some(s.to_owned())
    } else if s.starts_with("localhost") || s.starts_with("127.0.0.1") {
        Some(format!("http://{s}"))
    } else {
        Some(format!("https://{s}"))
    }
}

/// Sends `body` as JSON to `{server}/api{path}` and parses the JSON reply.
pub async fn call<B: Serialize, T: DeserializeOwned>(
    server: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&B>,
) -> Result<T, ApiError> {
    let init = RequestInit::new();
    init.set_method(method);
    let headers = Headers::new().map_err(js_err)?;
    headers.set("Accept", "application/json").map_err(js_err)?;
    if let Some(t) = token {
        headers.set("Authorization", &format!("Bearer {t}")).map_err(js_err)?;
    }
    if let Some(b) = body {
        let json = serde_json::to_string(b).map_err(|_| ApiError { message: "Couldn't encode the request.".into(), status: None })?;
        headers.set("Content-Type", "application/json").map_err(js_err)?;
        init.set_body(&JsValue::from_str(&json));
    }
    init.set_headers(&headers);

    let request = Request::new_with_str_and_init(&format!("{server}/api{path}"), &init).map_err(js_err)?;
    let response: Response =
        JsFuture::from(window().fetch_with_request(&request)).await.map_err(js_err)?.dyn_into().map_err(js_err)?;
    let text = JsFuture::from(response.text().map_err(js_err)?).await.map_err(js_err)?.as_string().unwrap_or_default();

    if !response.ok() {
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(str::to_owned))
            .unwrap_or_else(|| format!("The server answered {}.", response.status()));
        return Err(ApiError { message, status: Some(response.status()) });
    }
    serde_json::from_str(&text)
        .map_err(|_| ApiError { message: "The server sent something unexpected.".into(), status: Some(response.status()) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_server_addresses() {
        assert_eq!(normalize_server(" https://api.example.com/ ").as_deref(), Some("https://api.example.com"));
        assert_eq!(normalize_server("api.example.com").as_deref(), Some("https://api.example.com"));
        assert_eq!(normalize_server("localhost:5000").as_deref(), Some("http://localhost:5000"));
        assert_eq!(normalize_server("http://192.168.1.5:5000").as_deref(), Some("http://192.168.1.5:5000"));
        assert_eq!(normalize_server(""), None);
        assert_eq!(normalize_server("a b"), None);
    }
}
