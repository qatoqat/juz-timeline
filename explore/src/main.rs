mod card;
mod concept;
mod data;
mod nav;
mod outline;
mod compare;
mod ladder;
mod lanes;
mod relpanel;
mod relations;
mod revise;
mod slips;
mod trail;
mod ontology;
mod overview;
mod similar;
mod spine;
mod spread;
mod themes;
mod topics;
mod tree;

use dioxus::prelude::*;
use similar::SimilarAyah;
use themes::ThemesPage;
use topics::TopicsPage;

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
    experimental: bool,
}

const QF: &str = "https://quran.foundation";

const RELATIONS: [Item; 3] = [
    Item { name: "Similar ayah", arabic: "متشابهات", count: "4,072", scope: "phrase ↔ phrase", about: "Phrases repeated in 2–12 ayat, as word trees with the first word that differs, plus near-identical ayat.", slug: "similar-ayah", source: QF, experimental: false },
    Item { name: "Ayah themes", arabic: "مواضيع الآيات", count: "1,049", scope: "ayah range", about: "Core theme of each group of ayat, and where each passage is told or worded alike.", slug: "ayah-theme", source: "https://qul.tarteel.ai/resources/ayah-theme", experimental: false },
    Item { name: "Concepts", arabic: "المفاهيم", count: "284", scope: "concept ↔ concept", about: "Quranic Arabic Corpus concepts, the ayat they appear in, and how they relate.", slug: "ayah-topics", source: "https://qul.tarteel.ai/resources/ayah-topics", experimental: true },
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
    } else if path.ends_with("/ayah-topics") {
        rsx! { TopicsPage {} }
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
        main {
            header {
                h1 { "Explore" }
                a { class: "back", href: "../", "Juz timeline" }
            }
            section {
                h2 { "Relations" }
                p { class: "note",
                    "Links between ayat, phrases and words. Similar ayah is computed from the quran.com text; themes and concepts come from "
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
                                if it.experimental { span { class: "tag", "Experimental" } }
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
                " · concepts: "
                a { href: "https://corpus.quran.com/ontology.jsp", target: "_blank", rel: "noopener", "Quranic Arabic Corpus ontology ↗" }
                " (GNU GPL) via "
                a { href: "{QUL}/ayah-topics", target: "_blank", rel: "noopener", "QUL ↗" }
            }
        }
    }
}
