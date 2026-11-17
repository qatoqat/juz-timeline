mod card;
mod data;
mod nav;
mod overview;
mod similar;
mod spine;
mod themes;
mod tree;

use dioxus::prelude::*;
use similar::SimilarAyah;
use themes::ThemesPage;

const QUL: &str = "https://qul.tarteel.ai/resources";
const API: &str = "https://api-docs.quran.com";

#[derive(Clone, PartialEq)]
struct Item {
    name: &'static str,
    arabic: &'static str,
    count: &'static str,
    scope: &'static str,
    about: &'static str,
    slug: &'static str,
    ready: bool,
}

const RELATIONS: [Item; 4] = [
    Item { name: "Mutashabihat", arabic: "متشابهات", count: "5,278", scope: "phrase ↔ phrase", about: "Repeated phrases with small wording differences.", slug: "mutashabihat", ready: false },
    Item { name: "Similar ayah", arabic: "آيات متشابهة", count: "4,001", scope: "ayah ↔ ayah", about: "Ayat that match in meaning, context or wording, with a score.", slug: "similar-ayah", ready: true },
    Item { name: "Ayah themes", arabic: "مواضيع الآيات", count: "1,049", scope: "ayah range", about: "Core theme of each group of ayat.", slug: "ayah-theme", ready: true },
    Item { name: "Topics and concepts", arabic: "المفاهيم", count: "3,158", scope: "concept ↔ concept", about: "Key concepts, the ayat they appear in, and how they relate.", slug: "ayah-topics", ready: false },
];

fn main() {
    dioxus::launch(App);
}

fn path() -> String {
    web_sys::window().and_then(|w| w.location().pathname().ok()).unwrap_or_default()
}

fn go(href: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(href);
    }
}

#[component]
fn App() -> Element {
    let path = path();
    let path = path.trim_end_matches('/');
    if path.ends_with("/similar-ayah") {
        rsx! { SimilarAyah {} }
    } else if path.ends_with("/ayah-theme") {
        rsx! { ThemesPage {} }
    } else {
        rsx! { List {} }
    }
}

#[component]
fn Source(href: String) -> Element {
    rsx! {
        a { class: "source", href, target: "_blank", rel: "noopener", onclick: move |e| e.stop_propagation(), "Source ↗" }
    }
}

#[component]
fn List() -> Element {
    rsx! {
        style { {include_str!("../assets/explore.css")} }
        main {
            header {
                h1 { "Explore" }
                a { class: "back", href: "../", "Juz timeline" }
            }
            section {
                h2 { "Relations" }
                p { class: "note",
                    "Links between ayat, phrases and words, from "
                    a { href: "{QUL}", target: "_blank", rel: "noopener", "QUL ↗" }
                    ", quran.com's resource library. Divisions come from the "
                    a { href: "{API}", target: "_blank", rel: "noopener", "quran.com API ↗" }
                    "."
                }
                ul { class: "items",
                    for it in RELATIONS {
                        li {
                            class: if it.ready { "ready" } else { "soon" },
                            onclick: move |_| if it.ready { go(&format!("{}/", it.slug)) },
                            div { class: "head",
                                span { class: "name", "{it.name}" }
                                span { class: "ar", dir: "rtl", "{it.arabic}" }
                                span { class: "count", "{it.count}" }
                            }
                            div { class: "meta", span { class: "scope", "{it.scope}" } "{it.about}" }
                            div { class: "foot",
                                Source { href: format!("{QUL}/{}", it.slug) }
                                if !it.ready { span { class: "tag", "Soon" } }
                            }
                        }
                    }
                }
            }
        }
    }
}
