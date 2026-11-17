use crate::concept::Current;
use crate::ontology::SharedOntology;
use crate::spine::plural;
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{ayah_pos, range_of, AyahKey, Overlay, Selection};
use quran_ui::{Focus, MUSHAF};
use std::collections::HashMap;
use std::rc::Rc;

const GROUPS_PAGE: usize = 25;
const VIA_SHOWN: usize = 2;

#[derive(Clone)]
pub struct Texts(pub Option<Rc<HashMap<u16, String>>>);

impl PartialEq for Texts {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

fn juz_of(pos: f32) -> usize {
    MUSHAF.juz.partition_point(|j| j.start <= pos + 1e-4).saturating_sub(1)
}

fn ruku_of(pos: f32) -> usize {
    MUSHAF.ruku.partition_point(|r| r.pos <= pos + 1e-4).saturating_sub(1)
}

fn level(n: usize, max: usize) -> usize {
    if n == 0 { 0 } else { 1 + (n * 4 - 1) / max.max(1) }
}

pub fn pick(selection: Selection, overlay: Overlay, key: AyahKey) {
    let pos = ayah_pos(key);
    let inside = (*selection.selected.peek()).map(range_of).is_some_and(|(lo, hi)| pos >= lo && pos < hi);
    if inside {
        let mut picked = overlay.picked;
        picked.set(Some(key));
    } else {
        selection.show(key, overlay);
    }
}

/// How a concept's ayat spread over the 30 juz or the 114 surahs, right to left.
#[component]
pub fn Spread(onto: SharedOntology, i: usize, by_surah: Signal<bool>) -> Element {
    let current = use_context::<Current>();
    let selection = use_context::<Selection>();
    let o = onto.0.clone();
    let ayat = o.ayat(i, (current.rollup)());
    let selected = (selection.selected)();
    let cells: Vec<(usize, usize, f32, Focus, String)> = if by_surah() {
        let mut counts = vec![0; 114];
        for &a in ayat {
            counts[o.key(a).0 as usize - 1] += 1;
        }
        MUSHAF.surahs.iter().enumerate().map(|(s, m)| (s, counts[s], m.end - m.start, Focus::Surah(s), format!("{} {} · {}", m.num, m.name, plural(counts[s], "ayah")))).collect()
    } else {
        let mut counts = vec![0; 30];
        for &a in ayat {
            counts[juz_of(ayah_pos(o.key(a)))] += 1;
        }
        MUSHAF.juz.iter().enumerate().map(|(j, m)| (j, counts[j], m.end - m.start, Focus::Juz(j), format!("Juz {} · {}", m.num, plural(counts[j], "ayah")))).collect()
    };
    let max = cells.iter().map(|c| c.1).max().unwrap_or(0);
    let spread = cells.iter().filter(|c| c.1 > 0).count();
    let unit = if by_surah() { "surahs" } else { "juz" };

    rsx! {
        div { class: "spread-wrap",
            div { class: "spread-head",
                span { "In {spread} of {cells.len()} {unit}" }
                div { class: "toggle small",
                    button { class: if !by_surah() { "on" }, onclick: move |_| by_surah.set(false), "Juz" }
                    button { class: if by_surah() { "on" }, onclick: move |_| by_surah.set(true), "Surah" }
                }
            }
            div { class: if by_surah() { "spread surahs" } else { "spread" }, dir: "rtl",
                for (n, count, width, focus, tip) in cells {
                    button {
                        key: "{unit}{n}",
                        class: format!("scell l{}{}", level(count, max), if selected == Some(focus) { " on" } else { "" }),
                        style: "flex-grow:{width}",
                        "data-tip": "{tip}",
                        onclick: move |_| {
                            let mut s = selection.selected;
                            s.set(Some(focus));
                        },
                        if !by_surah() { "{n + 1}" }
                    }
                }
            }
        }
    }
}

/// The concept's ayat grouped by ruku', with their text once loaded.
#[component]
pub fn Occurrences(onto: SharedOntology, i: usize, texts: Texts) -> Element {
    let current = use_context::<Current>();
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let mut pages = use_signal(|| 1usize);
    let picked = (overlay.picked)();
    use_effect(move || {
        if let Some((s, a)) = (overlay.picked)() {
            let _ = document::eval(&format!("document.getElementById('occ-{s}-{a}')?.scrollIntoView({{block:'nearest',behavior:'smooth'}})"));
        }
    });
    let o = onto.0.clone();
    let rollup = (current.rollup)();
    let mut groups: Vec<(usize, Vec<u16>)> = vec![];
    for &a in o.ayat(i, rollup) {
        let r = ruku_of(ayah_pos(o.key(a)));
        match groups.last_mut() {
            Some(g) if g.0 == r => g.1.push(a),
            _ => groups.push((r, vec![a])),
        }
    }
    let total = groups.len();
    let reach = picked.and_then(|key| o.slot(key)).and_then(|a| groups.iter().position(|g| g.1.contains(&a))).map_or(0, |at| at / GROUPS_PAGE + 1);
    let shown = (pages().max(reach) * GROUPS_PAGE).min(total);
    groups.truncate(shown);

    rsx! {
        div { class: "occ-list",
            for (r, members) in groups {
                {
                    let ruku = &MUSHAF.ruku[r];
                    let in_surah = MUSHAF.ruku[..=r].iter().rev().take_while(|k| k.surah == ruku.surah).count();
                    let surah = &MUSHAF.surahs[ruku.surah as usize - 1];
                    rsx! {
                        div { key: "g{r}", class: "ruku-group",
                            p { class: "ruku-head",
                                "{surah.num} · " span { class: "ar", "{surah.name}" }
                                " · Ruku' {in_surah} · page {ruku.pos.floor() as u32 + 1}"
                            }
                            for a in members {
                                {
                                    let key = o.key(a);
                                    let (s, n) = key;
                                    let via: Vec<String> = if rollup { o.via(i, a).into_iter().take(VIA_SHOWN).map(|k| o.title(k).to_string()).collect() } else { vec![] };
                                    let text = texts.0.as_ref().and_then(|t| t.get(&a).cloned());
                                    rsx! {
                                        div {
                                            key: "{a}",
                                            id: "occ-{s}-{n}",
                                            class: if picked == Some(key) { "occ on" } else { "occ" },
                                            onmouseenter: move |_| {
                                                let mut hot = overlay.hot;
                                                hot.set(vec![key]);
                                            },
                                            onmouseleave: move |_| {
                                                let mut hot = overlay.hot;
                                                hot.set(vec![]);
                                            },
                                            onclick: move |_| pick(selection, overlay, key),
                                            oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(s, n)),
                                            span { class: "ref", "{s}:{n}" }
                                            span { class: if text.is_some() { "occ-text" } else { "occ-text wait" }, dir: "rtl", {text.clone().unwrap_or_else(|| "…".to_string())} }
                                            if !via.is_empty() { span { class: "occ-via", dir: "rtl", "{via.join(\" · \")}" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if shown < total {
                button { class: "chips-more", onclick: move |_| pages.set(shown / GROUPS_PAGE + 1), "Show more ({total - shown} ruku' left)" }
            }
        }
    }
}
