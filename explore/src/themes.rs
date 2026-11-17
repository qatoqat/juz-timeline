use dioxus::prelude::*;
use quran_ui::band::{Band, Cell};
use quran_ui::overlay::{ayah_pos, range_of, Selection};
use quran_ui::{Focus, MUSHAF};
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/ayah-theme";
const FETCH: &str = r#"fetch("themes.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const SEARCH_LIMIT: usize = 30;

#[derive(Clone, PartialEq)]
struct Theme {
    kind: u8,
    surah: u8,
    from: u16,
    to: u16,
    text: String,
    number: usize,
    ruku: usize,
    start: f32,
    end: f32,
}

fn surah_end(surah: u8) -> f32 {
    MUSHAF.surahs[surah as usize - 1].end
}

fn ruku_of(surah: u8, ayah: u16) -> usize {
    MUSHAF.ruku.iter().rposition(|r| (r.surah, r.ayah) <= (surah, ayah)).unwrap_or(0)
}

fn parse(json: &str) -> Option<Vec<Theme>> {
    let raw: Vec<(u8, u16, u16, u8, String)> = serde_json::from_str(json).ok()?;
    let mut number = 0;
    let mut last = 0;
    Some(
        raw.into_iter()
            .map(|(surah, from, to, kind, text)| {
                number = if surah == last { number + 1 } else { 1 };
                last = surah;
                let ayat = MUSHAF.surahs[surah as usize - 1].ayat;
                let end = if to >= ayat { surah_end(surah) } else { ayah_pos((surah, to + 1)) };
                Theme { kind, surah, from, to, text, number, ruku: ruku_of(surah, from), start: ayah_pos((surah, from)), end }
            })
            .collect(),
    )
}

const NO_THEME: u8 = 2;
const CONTINUED: u8 = 1;

fn describe(t: &Theme) -> String {
    match t.kind {
        NO_THEME => format!("No theme · opens with {}", t.text),
        CONTINUED => format!("{} (continued)", t.text),
        _ => t.text.clone(),
    }
}

fn cells(themes: &[Theme]) -> Vec<Cell> {
    themes
        .iter()
        .enumerate()
        .map(|(id, t)| {
            let r = &MUSHAF.ruku[t.ruku];
            Cell {
                id,
                start: t.start,
                end: t.end,
                number: t.number,
                label: describe(t),
                tip: format!("{}:{}–{} · {}", t.surah, t.from, t.to, describe(t)),
                heavy: (r.surah, r.ayah) == (t.surah, t.from),
                level: 0,
            }
        })
        .collect()
}

#[derive(Clone)]
struct Shared(Rc<Vec<Theme>>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
pub fn ThemesPage() -> Element {
    let mut themes = use_signal(|| None::<Rc<Vec<Theme>>>);
    let band = use_context_provider(|| Band { cells: Signal::new(vec![]), hovered: Signal::new(None), chosen: Signal::new(None) });
    use_future(move || async move {
        let mut fetch = document::eval(FETCH);
        if let Ok(text) = fetch.recv::<String>().await {
            if let Some(list) = parse(&text) {
                let mut cells_signal = band.cells;
                cells_signal.set(cells(&list));
                themes.set(Some(Rc::new(list)));
            }
        }
    });

    rsx! {
        style { {include_str!("../assets/themes.css")} }
        quran_ui::Page {
            ruku: false,
            title: rsx! {
                h1 {
                    "مواضيع الآيات"
                    span { class: "sub", "Ayah themes" }
                    a { class: "explore", href: "../", "Explore" }
                    a { class: "explore", href: "{QUL}", target: "_blank", rel: "noopener", "Source ↗" }
                }
            },
            extra: rsx! { Outline { themes: themes().map(Shared) } },
        }
    }
}

fn show(selection: Selection, band: Band, t: &Theme, id: usize) {
    let mut selected = selection.selected;
    let mut chosen = band.chosen;
    selected.set(Some(Focus::Surah(t.surah as usize - 1)));
    chosen.set(Some(id));
}

#[component]
fn Outline(themes: Option<Shared>) -> Element {
    let selection = use_context::<Selection>();
    let band = use_context::<Band>();
    let mut query = use_signal(String::new);
    let mut hide = use_signal(|| false);
    let mut shown = use_signal(Vec::<usize>::new);
    let (mut hovered, chosen) = (band.hovered, band.chosen);
    use_effect(move || {
        if let Some(id) = chosen() {
            let _ = document::eval(&format!("document.getElementById('theme-{id}')?.scrollIntoView({{block:'nearest',behavior:'smooth'}})"));
        }
    });
    let Some(Shared(list)) = themes else { return rsx! { p { class: "themes-note", "Loading themes…" } } };
    let q = query().trim().to_lowercase();
    let range = (selection.selected)().map(range_of);
    let ids: Vec<usize> = if q.chars().count() >= 2 {
        list.iter().enumerate().filter(|(_, t)| t.text.to_lowercase().contains(&q)).map(|(i, _)| i).take(SEARCH_LIMIT).collect()
    } else if let Some((lo, hi)) = range {
        list.iter().enumerate().filter(|(_, t)| t.start < hi - 1e-3 && t.end > lo + 1e-3).map(|(i, _)| i).collect()
    } else {
        vec![]
    };
    let searching = q.chars().count() >= 2;
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
                button {
                    class: if hide() { "drill on" } else { "drill" },
                    "data-tip": "Recall the sequence of themes: text hides until you tap it",
                    onclick: move |_| {
                        hide.toggle();
                        shown.set(vec![]);
                    },
                    "Hide text"
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
                                    let show_t = t.clone();
                                    let revealed = !hide() || shown.read().contains(&id);
                                    let pages = ((t.end - t.start) * 10.0).round() / 10.0;
                                    rsx! {
                                        div {
                                            key: "{id}",
                                            id: "theme-{id}",
                                            class: format!("theme-row{}{}", if hovered() == Some(id) { " hot" } else { "" }, if chosen() == Some(id) { " on" } else { "" }),
                                            onmouseenter: move |_| hovered.set(Some(id)),
                                            onmouseleave: move |_| hovered.set(None),
                                            onclick: move |_| {
                                                if !revealed {
                                                    shown.write().push(id);
                                                }
                                                show(selection, band, &show_t, id);
                                            },
                                            span { class: "num", "{t.number}" }
                                            span { class: "span", "{t.surah}:{t.from}–{t.to}" }
                                            div { class: format!("text{}{}", if revealed { "" } else { " hidden" }, if t.kind == NO_THEME { " none" } else { "" }), "{describe(&t)}" }
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
