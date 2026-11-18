mod card;
mod data;
mod nav;
mod outline;
mod compare;
mod ladder;
mod lanes;
mod relpanel;
mod relations;
mod revise;
mod trail;
mod overview;
mod similar;
mod spine;
mod themes;
mod tree;
#[cfg(test)]
mod tests;

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
    source: &'static str,
}

const QF: &str = "https://quran.foundation";

const RELATIONS: [Item; 2] = [
    Item { name: "Similar ayah", arabic: "متشابهات", count: "4,071", scope: "phrase ↔ phrase", about: "Phrases repeated in 2–12 ayat, as word trees with the first word that differs, plus near-identical ayat.", slug: "similar-ayah", source: QF },
    Item { name: "Ayah themes", arabic: "مواضيع الآيات", count: "1,049", scope: "ayah range", about: "Core theme of each group of ayat, and where each passage is told or worded alike.", slug: "ayah-theme", source: "https://qul.tarteel.ai/resources/ayah-theme" },
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
    let page = if path.ends_with("/similar-ayah") {
        rsx! { SimilarAyah {} }
    } else if path.ends_with("/ayah-theme") {
        rsx! { ThemesPage {} }

    } else {
        rsx! { List {} }
    };
    rsx! {
        {page}
        quran_ui::Stale {}
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
        style { {quran_ui::SCROLLBARS} }
        main {
            header {
                h1 { "Explore" }
                a { class: "back", href: "../", "Juz timeline" }
            }
            section {
                h2 { "Relations" }
                p { class: "note",
                    "Links between ayat, phrases and words. Similar ayah is computed from the quran.com text; themes come from "
                    a { href: "{QUL}", target: "_blank", rel: "noopener", "QUL ↗" }
                    ", quran.com's resource library. Divisions come from the "
                    a { href: "{API}", target: "_blank", rel: "noopener", "quran.com API ↗" }
                    "."
                }
                ul { class: "items",
                    for it in RELATIONS {
                        li {
                            class: "ready",
                            onclick: move |_| go(&format!("{}/", it.slug)),
                            div { class: "head",
                                span { class: "name", "{it.name}" }
                                span { class: "ar", dir: "rtl", "{it.arabic}" }
                                span { class: "count", "{it.count}" }
                            }
                            div { class: "meta", span { class: "scope", "{it.scope}" } "{it.about}" }
                            div { class: "foot",
                                Source { href: it.source }
                            }
                        }
                    }
                }
            }
            footer { class: "sources",
                "Quran text and data: "
                a { href: "https://quran.foundation", target: "_blank", rel: "noopener", "Quran Foundation / quran.com ↗" }
                " (synced {quran_ui::data::DATA_FETCHED}; page layout: King Fahd Complex 1441 AH print) · ayah themes: Muhammad Farooq-i-Azam Malik's ruku' headings via "
                a { href: "{QUL}/ayah-theme", target: "_blank", rel: "noopener", "QUL ↗" }
                " · people and things linking theme passages: "
                a { href: "https://corpus.quran.com/ontology.jsp", target: "_blank", rel: "noopener", "Quranic Arabic Corpus ontology ↗" }
                " (GNU GPL) via "
                a { href: "{QUL}/ayah-topics", target: "_blank", rel: "noopener", "QUL ↗" }
            }
        }
    }
}
