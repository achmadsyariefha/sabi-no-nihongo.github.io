//! Saving files to the user's device and reading files they pick.

use std::time::Duration;

use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, BlobPropertyBag, File, HtmlAnchorElement, HtmlInputElement, Url};

/// Starts a download of `content` as `filename`.
pub fn download(filename: &str, mime: &str, content: &str) -> Result<(), JsValue> {
    let parts = js_sys::Array::of1(&JsValue::from_str(content));
    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &opts)?;
    let url = Url::create_object_url_with_blob(&blob)?;

    let link: HtmlAnchorElement = document().create_element("a")?.unchecked_into();
    link.set_href(&url);
    link.set_download(filename);
    link.set_hidden(true);
    document().body().ok_or("no <body>")?.append_child(&link)?;
    link.click();
    link.remove();
    // Give the browser a moment to start the download before freeing the data.
    set_timeout(move || drop(Url::revoke_object_url(&url)), Duration::from_secs(10));
    Ok(())
}

/// The file picked in an `<input type="file">`, if any. Clears the input so
/// picking the same file again still fires `change`.
pub fn take_picked_file(input: &HtmlInputElement) -> Option<File> {
    let file = input.files().and_then(|list| list.get(0));
    input.set_value("");
    file
}

/// Reads a file as UTF-8 text.
pub async fn read_text(file: &File) -> Result<String, JsValue> {
    JsFuture::from(file.text()).await?.as_string().ok_or_else(|| "not text".into())
}
