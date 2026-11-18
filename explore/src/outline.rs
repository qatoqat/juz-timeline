use crate::themes::{describe, open, Themes, View, NO_THEME};
use dioxus::prelude::*;
use quran_ui::band::Band;
use quran_ui::overlay::{range_of, Selection};
use quran_ui::MUSHAF;

const SEARCH_LIMIT: usize = 30;

#[component]
pub fn Outline(themes: Themes) -> Element {
    let selection = use_context::<Selection>();
    let band = use_context::<Band>();
    let view = use_context::<View>();
    let mut query = use_signal(String::new);
    let (mut hovered, chosen) = (band.hovered, band.chosen);
    use_effect(move || {
        if let Some(id) = chosen() {
            let _ = document::eval(&format!("document.getElementById('theme-{id}')?.scrollIntoView({{block:'nearest',behavior:'smooth'}})"));
        }
    });
    let list = themes.0.clone();
    let rel = (view.rel)();
    let q = query().trim().to_lowercase();
    let range = (selection.selected)().map(range_of);
    let searching = q.chars().count() >= 2;
    let ids: Vec<usize> = if searching {
        list.iter().enumerate().filter(|(_, t)| t.text.to_lowercase().contains(&q)).map(|(i, _)| i).take(SEARCH_LIMIT).collect()
    } else if let Some((lo, hi)) = range {
        list.iter().enumerate().filter(|(_, t)| t.start < hi - 1e-3 && t.end > lo + 1e-3).map(|(i, _)| i).collect()
    } else {
        vec![]
    };
    let mut groups: Vec<(usize, Vec<usize>)> = vec![];
    for id in ids {
        let r = list[id].ruku;
        match groups.last_mut() {
            Some(g) if g.0 == r && !searching => g.1.push(id),
            _ => groups.push((r, vec![id])),
        }
    }

    rsx! {
        div { class: "outline",
            div { class: "outline-bar",
                input {
                    class: "theme-search",
                    r#type: "search",
                    placeholder: "Search themes",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
            }
            if groups.is_empty() {
                p { class: "themes-note",
                    if searching { "No theme matches." } else { "Select a surah or juz to see its themes, or search above." }
                }
            }
            div { class: "outline-list",
                for (r, members) in groups {
                    {
                        let ruku = &MUSHAF.ruku[r];
                        let in_surah = MUSHAF.ruku[..=r].iter().rev().take_while(|k| k.surah == ruku.surah).count();
                        let surah = &MUSHAF.surahs[ruku.surah as usize - 1];
                        rsx! {
                            div { key: "r{r}", class: "ruku-group",
                                p { class: "ruku-head",
                                    if searching { "{surah.num} · " span { class: "ar", "{surah.name}" } " · " }
                                    "Ruku' {in_surah} · from {ruku.surah}:{ruku.ayah} · page {ruku.pos.floor() as u32 + 1}"
                                }
                                for id in members {
                                    {
                                        let t = list[id].clone();
                                        let open_list = list.clone();
                                        let pages = ((t.end - t.start) * 10.0).round() / 10.0;
                                        let (twins, links) = rel.as_ref().map_or((0, 0), |r| (r.twins_of[id].len(), r.links_of[id].len()));
                                        rsx! {
                                            div {
                                                key: "{id}",
                                                id: "theme-{id}",
                                                class: format!("theme-row{}{}", if hovered() == Some(id) { " hot" } else { "" }, if chosen() == Some(id) { " on" } else { "" }),
                                                onmouseenter: move |_| hovered.set(Some(id)),
                                                onmouseleave: move |_| hovered.set(None),
                                                onclick: move |_| open(view, band, selection, &open_list, id, None),
                                                span { class: "num", "{t.number}" }
                                                span { class: "span", "{t.surah}:{t.from}–{t.to}" }
                                                div { class: if t.kind == NO_THEME { "text none" } else { "text" },
                                                    "{describe(&t)}"
                                                    if twins > 0 { span { class: "rel-badge twin", "data-tip": "Look-alike passages", "≈{twins}" } }
                                                    if links > 0 { span { class: "rel-badge", "data-tip": "Linked passages", "↔{links}" } }
                                                }
                                                span { class: "len", "{pages} p" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
