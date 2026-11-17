use dioxus::prelude::*;
use quran_core::data::{JUZ, MANZIL, PAGES, RUB, SAJDAH, SURAHS};

const QUL: &str = "https://qul.tarteel.ai/resources";
const API: &str = "https://api-docs.quran.com";

#[derive(Clone, PartialEq)]
struct Item {
    name: &'static str,
    arabic: &'static str,
    count: String,
    scope: &'static str,
    about: &'static str,
    link: String,
}

fn item(name: &'static str, arabic: &'static str, count: impl ToString, scope: &'static str, about: &'static str, link: String) -> Item {
    Item { name, arabic, count: count.to_string(), scope, about, link }
}

fn divisions() -> Vec<Item> {
    vec![
        item("Surah", "سورة", SURAHS.len(), "chapter", "Revelation place (Makki / Madani), revelation order, page range, bismillah.", API.to_string()),
        item("Juz", "جزء", JUZ.len() - 1, "per ayah", "Thirty equal parts for monthly reading.", API.to_string()),
        item("Hizb", "حزب", (RUB.len() - 1) / 4, "per ayah", "Two per juz.", API.to_string()),
        item("Rub' al-hizb", "ربع الحزب", RUB.len() - 1, "per ayah", "Quarter of a hizb, the usual daily memorisation unit.", API.to_string()),
        item("Manzil", "منزل", MANZIL.len(), "per ayah", "Seven parts for weekly reading (\u{2068}فمي بشوق\u{2069}).", API.to_string()),
        item("Ruku'", "ركوع", 556, "per ayah", "Thematic paragraph breaks, marked ع in South Asian mushafs.", API.to_string()),
        item("Page", "صفحة", PAGES.len() - 1, "per ayah", "Madani mushaf page, with 15-line layout data.", format!("{QUL}/mushaf-layout")),
        item("Sajdah", "سجدة", SAJDAH.len(), "per ayah", "Ayat of prostration.", API.to_string()),
    ]
}

fn relations() -> Vec<Item> {
    vec![
        item("Mutashabihat", "متشابهات", "5,278", "phrase ↔ phrase", "Repeated phrases with small wording differences.", format!("{QUL}/mutashabihat")),
        item("Similar ayah", "آيات متشابهة", "4,001", "ayah ↔ ayah", "Ayat that match in meaning, context or wording, with a score.", format!("{QUL}/similar-ayah")),
        item("Ayah themes", "مواضيع الآيات", "1,049", "ayah range", "Core theme of each group of ayat.", format!("{QUL}/ayah-theme")),
        item("Topics and concepts", "المفاهيم", "3,158", "concept ↔ concept", "Key concepts, the ayat they appear in, and how they relate.", format!("{QUL}/ayah-topics")),
        item("Surah info", "تعريف السورة", "114", "per surah", "Background, period of revelation and main themes.", format!("{QUL}/surah-info")),
        item("Word roots", "الجذور", "77,432", "word ↔ root", "Morphology: root, lemma and stem of every word.", format!("{QUL}/morphology")),
        item("Tafsir grouping", "تفسير", "115", "ayah range", "How each tafsir groups ayat when explaining them.", format!("{QUL}/tafsir")),
    ]
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn List(title: &'static str, note: &'static str, items: Vec<Item>) -> Element {
    rsx! {
        section {
            h2 { "{title}" }
            p { class: "note", "{note}" }
            ul { class: "items",
                for it in items {
                    li {
                        div { class: "head",
                            span { class: "name", "{it.name}" }
                            span { class: "ar", dir: "rtl", "{it.arabic}" }
                            span { class: "count", "{it.count}" }
                        }
                        div { class: "meta", span { class: "scope", "{it.scope}" } "{it.about}" }
                        a { href: "{it.link}", target: "_blank", rel: "noopener", "Source" }
                    }
                }
            }
        }
    }
}

#[component]
fn App() -> Element {
    rsx! {
        style { {include_str!("../assets/explore.css")} }
        main {
            header {
                h1 { "Explore" }
                a { class: "back", href: "../", "Juz timeline" }
            }
            List {
                title: "Divisions",
                note: "How the mushaf is split. Every ayah carries these numbers in the quran.com API.",
                items: divisions(),
            }
            List {
                title: "Relations",
                note: "Links between ayat, phrases and words, from QUL (quran.com's resource library).",
                items: relations(),
            }
        }
    }
}
