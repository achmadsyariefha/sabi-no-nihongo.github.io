//! A short burst of CSS confetti. Fired through `AppState::fire_confetti`.

use std::time::Duration;

use leptos::prelude::*;

use crate::services::theme;
use crate::state::use_app_state;

const PIECES: usize = 80;
const COLORS: [&str; 5] = ["#f43f75", "#ffa8c1", "#3b82f6", "#10b981", "#fbbf24"];

struct Piece {
    style: String,
}

fn burst() -> Vec<Piece> {
    let r = js_sys::Math::random;
    (0..PIECES)
        .map(|i| {
            // Start around the middle of the screen and fly outwards, then fall.
            let left = 35.0 + r() * 30.0;
            let dx = (r() - 0.5) * 70.0;
            let rise = 20.0 + r() * 35.0;
            let style = format!(
                "left:{left:.1}vw;top:60vh;background:{};--dx:{dx:.1}vw;--rise:-{rise:.1}vh;--rot:{:.0}deg;--dur:{:.2}s;--delay:{:.2}s;{}",
                COLORS[i % COLORS.len()],
                (r() - 0.5) * 1080.0,
                1.6 + r() * 1.0,
                r() * 0.15,
                if i % 3 == 0 { "border-radius:9999px;width:7px;height:7px" } else { "" },
            );
            Piece { style }
        })
        .collect()
}

#[component]
pub fn Confetti() -> impl IntoView {
    let state = use_app_state();
    let pieces = RwSignal::new(Vec::<Piece>::new());
    let reduced = theme::prefers_reduced_motion();

    Effect::new(move |prev: Option<u32>| {
        let n = state.celebrate.get();
        // Fire on changes only, not on the first run.
        if prev.is_some() && !reduced {
            pieces.set(burst());
            set_timeout(
                move || {
                    if state.celebrate.get_untracked() == n {
                        pieces.set(Vec::new());
                    }
                },
                Duration::from_secs(3),
            );
        }
        n
    });

    view! {
        <div class="pointer-events-none fixed inset-0 z-[60] overflow-hidden" aria-hidden="true">
            {move || {
                pieces.with(|list| list.iter().map(|p| view! { <span class="confetti-piece" style=p.style.clone()></span> }).collect_view())
            }}
        </div>
    }
}
