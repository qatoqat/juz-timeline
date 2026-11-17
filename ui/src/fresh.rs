use dioxus::prelude::*;

use crate::data::DATA_FETCHED;

/// The Quran Foundation developer terms allow offline copies up to 7 days old.
const MAX_AGE_DAYS: f64 = 7.0;

const CSS: &str = ".stale { position: fixed; left: 16px; bottom: 16px; z-index: 20; width: 26px; height: 26px; border-radius: 50%;
  display: grid; place-items: center; font: 600 15px/1 system-ui, sans-serif; color: #1a1a1a; background: #e8b53c; cursor: help; }
.stale::after { content: attr(aria-label); position: absolute; left: 0; bottom: 34px; width: max-content; max-width: min(320px, calc(100vw - 32px));
  padding: 6px 10px; border-radius: 6px; font-weight: 400; font-size: 13px; line-height: 1.45; color: #eee; background: #222;
  box-shadow: 0 2px 8px #0006; opacity: 0; pointer-events: none; transition: opacity .12s; }
.stale:hover::after, .stale:focus::after { opacity: 1; }";

fn age_days() -> f64 {
    (js_sys::Date::now() - js_sys::Date::parse(DATA_FETCHED)) / 86_400_000.0
}

/// A warning icon, shown only when the synced data is older than the terms allow.
#[component]
pub fn Stale() -> Element {
    let age = age_days();
    if age.is_nan() || age <= MAX_AGE_DAYS {
        return VNode::empty();
    }
    let label = format!(
        "Quran data last synced {DATA_FETCHED} ({} days ago); the scheduled refresh has not run since.",
        age.floor()
    );
    rsx! {
        style { {CSS} }
        span { class: "stale", role: "status", tabindex: "0", "aria-label": "{label}", "!" }
    }
}
