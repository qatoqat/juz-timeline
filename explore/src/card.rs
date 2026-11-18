use crate::data::Row;
use crate::tree::{ayah_label, light, surah_name, unlight, Shared};
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{Overlay, Selection};

#[component]
fn Words(words: Vec<String>, cue: Option<usize>, lead: bool, trail: bool) -> Element {
    rsx! {
        if lead { span { class: "ell", "… " } }
        for (i, w) in words.into_iter().enumerate() {
            span { key: "{i}", class: if cue == Some(i) { "w cue" } else { "w" }, "{w} " }
        }
        if trail { span { class: "ell", "…" } }
    }
}

#[component]
pub fn Card(data: Shared, family: usize, picked: usize) -> Element {
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let d = data.0.clone();
    let rows: Vec<Row> = d.rows(family);
    let phrase = if d.families[family].members.iter().any(|m| m.0 == picked) {
        let (start, end) = d.span_in(family, picked);
        d.words(picked)[start..end].join(" ")
    } else {
        d.phrase(family)
    };

    rsx! {
        div { class: "card",
            div { class: "card-head",
                span { class: "phrase", dir: "rtl", "{phrase}" }
                span { class: "badge", "×{rows.len()}" }
                span { class: "hint", "highlighted: the first word that differs" }
            }
            div { class: "card-grid", dir: "rtl",
                for row in rows {
                    {
                        let a = row.ayah;
                        let key = d.keys[a];
                        let (enter_d, ayat) = (d.clone(), vec![a]);
                        rsx! {
                            div {
                                key: "{a}",
                                class: if a == picked { "card-row self" } else { "card-row" },
                                onmouseenter: move |_| light(overlay, &enter_d, &ayat),
                                onmouseleave: move |_| unlight(overlay),
                                span { class: "cb", Words { words: row.before.clone(), cue: row.before_cue, lead: row.before_more, trail: false } }
                                span { class: "cm", "data-tip": "{phrase}", "◆" }
                                span { class: "ca",
                                    if row.after.is_empty() { span { class: "w cue", "۝" } }
                                    Words { words: row.after.clone(), cue: row.after_cue, lead: false, trail: row.after_more }
                                }
                                button {
                                    class: if a == picked { "ref self" } else { "ref" },
                                    "data-tip": "{ayah_label(&d, a)} · {surah_name(&d, a)}",
                                    onclick: move |_| selection.show(key, overlay),
                                    oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(key.0, key.1)),
                                    "{ayah_label(&d, a)} · {surah_name(&d, a)}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
