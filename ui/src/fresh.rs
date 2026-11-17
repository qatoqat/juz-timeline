use dioxus::prelude::*;

use crate::data::DATA_FETCHED;

/// The Quran Foundation developer terms allow offline copies up to 7 days old.
const MAX_AGE_DAYS: f64 = 7.0;

pub fn age_days() -> f64 {
    let fetched = js_sys::Date::parse(DATA_FETCHED);
    if fetched.is_nan() {
        return f64::INFINITY;
    }
    (js_sys::Date::now() - fetched) / 86_400_000.0
}

/// Renders its children only while the synced data is fresh enough.
#[component]
pub fn Fresh(children: Element) -> Element {
    if age_days() <= MAX_AGE_DAYS {
        return children;
    }
    rsx! {
        main { style: "max-width: 36rem; margin: 20vh auto; padding: 0 16px; font: 16px/1.6 system-ui, sans-serif;",
            h1 { style: "font-size: 1.3rem;", "Temporarily unavailable" }
            p { "The Quran data on this site was last synced on {DATA_FETCHED}. It is hidden until the next sync, because the Quran Foundation terms allow copies up to 7 days old." }
            p { "Read the Quran at "
                a { href: "https://quran.com", "quran.com" }
                " meanwhile."
            }
        }
    }
}
