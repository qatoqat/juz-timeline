use dioxus::prelude::*;

use crate::data::DATA_FETCHED;

/// The Quran Foundation developer terms allow offline copies up to 7 days old.
const MAX_AGE_DAYS: f64 = 7.0;

const CSS: &str = ".stale { position: fixed; top: 12px; left: 50%; transform: translateX(-50%); z-index: 30; display: flex; align-items: center; gap: 10px;
  width: max-content; max-width: calc(100vw - 32px); padding: 8px 8px 8px 12px; border-radius: 8px;
  font: 13px/1.45 system-ui, sans-serif; color: #eee; background: #222; box-shadow: 0 4px 14px #0006; }
.stale b { color: #e8b53c; }
.stale button { font: inherit; font-size: 16px; line-height: 1; padding: 2px 6px; border: 0; border-radius: 4px; color: #aaa; background: transparent; cursor: pointer; }
.stale button:hover { color: #fff; background: #333; }";

fn age_days() -> f64 {
    (js_sys::Date::now() - js_sys::Date::parse(DATA_FETCHED)) / 86_400_000.0
}

/// A dismissible toast, shown only when the synced data is older than the terms allow.
#[component]
pub fn Stale() -> Element {
    let mut open = use_signal(|| true);
    let age = age_days();
    if !open() || age.is_nan() || age <= MAX_AGE_DAYS {
        return VNode::empty();
    }
    rsx! {
        style { {CSS} }
        div { class: "stale", role: "status",
            span {
                b { "Out of date: " }
                "Quran data last synced {DATA_FETCHED} ({age.floor()} days ago)."
            }
            button { "aria-label": "Dismiss", onclick: move |_| open.set(false), "×" }
        }
    }
}
