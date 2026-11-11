//! Japanese text-to-speech through the browser's Web Speech API.

use std::cell::Cell;
use std::rc::Rc;

use leptos::prelude::window;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{SpeechSynthesisUtterance, SpeechSynthesisVoice};

pub fn is_supported() -> bool {
    window().speech_synthesis().is_ok()
}

/// Speaks `text` in Japanese, slightly slower than normal for learning.
/// `on_end` runs once when speech finishes, fails, or isn't available.
pub fn speak(text: &str, on_end: impl FnOnce() + 'static) {
    type Once = Option<Box<dyn FnOnce()>>;
    let done: Rc<Cell<Once>> = Rc::new(Cell::new(Some(Box::new(on_end))));
    let finish = {
        let done = done.clone();
        move || {
            if let Some(f) = done.take() {
                f();
            }
        }
    };

    let Ok(synth) = window().speech_synthesis() else {
        return finish();
    };
    let Ok(utterance) = SpeechSynthesisUtterance::new_with_text(text) else {
        return finish();
    };
    utterance.set_lang("ja-JP");
    utterance.set_rate(0.9);

    let voice = synth
        .get_voices()
        .iter()
        .filter_map(|v| v.dyn_into::<SpeechSynthesisVoice>().ok())
        .find(|v| v.lang() == "ja-JP" || v.lang().starts_with("ja"));
    if let Some(voice) = voice {
        utterance.set_voice(Some(&voice));
    }

    let on_end_cb = Closure::<dyn FnMut()>::new(finish.clone()).into_js_value();
    let on_error_cb = Closure::<dyn FnMut()>::new(finish).into_js_value();
    utterance.set_onend(Some(on_end_cb.unchecked_ref()));
    utterance.set_onerror(Some(on_error_cb.unchecked_ref()));

    synth.cancel();
    synth.speak(&utterance);
}
