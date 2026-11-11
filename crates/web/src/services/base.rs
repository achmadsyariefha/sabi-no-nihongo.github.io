//! The path the app is served from, read from the `<base>` tag Trunk writes.
//!
//! `/` locally, `/sabi-no-nihongo.github.io/` on a GitHub project Pages site.

use leptos::prelude::document;

/// Router base without a trailing slash: `""` at the domain root,
/// `"/sabi-no-nihongo.github.io"` under a subpath.
pub fn router_base() -> String {
    let path = document()
        .base_uri()
        .ok()
        .flatten()
        .and_then(|uri| web_sys::Url::new(&uri).ok())
        .map(|url| url.pathname())
        .unwrap_or_default();
    normalize(&path)
}

/// Full link for an app path like `"/vault"`, including the base.
///
/// Needed because `<A href="/vault">` is used as-is by leptos_router and
/// would skip the base on a subpath site.
pub fn app_href(path: &str) -> String {
    format!("{}{}", router_base(), path)
}

fn normalize(path: &str) -> String {
    path.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn strips_trailing_slash() {
        assert_eq!(normalize("/"), "");
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("/sabi-no-nihongo.github.io/"), "/sabi-no-nihongo.github.io");
    }
}
