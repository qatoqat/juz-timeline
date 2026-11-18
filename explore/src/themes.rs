use crate::data::Data;
use crate::lanes::use_lanes;
use crate::outline::Outline;
use crate::relations::{index_of, Relations};
use crate::relpanel::ThemePanel;
use crate::revise::Revise;
use crate::trail::Trail;
use dioxus::prelude::*;
use quran_ui::arcs::Arcs;
use quran_ui::band::{Band, Cell};
use quran_ui::overlay::{ayah_pos, range_of, Overlay, Selection};
use quran_ui::{Focus, MUSHAF};
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/ayah-theme";
const FETCH: &str = r#"fetch("themes.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const FETCH_RELATIONS: &str = r#"fetch("relations.json").then(r => r.ok ? r.text() : "").then(t => dioxus.send(t)).catch(() => dioxus.send(""))"#;
const FETCH_SIMILAR: &str = r#"fetch("../similar-ayah/similar.json", {cache: CACHE}).then(r => r.text()).then(t => dioxus.send(t)).catch(() => dioxus.send(""))"#;

#[derive(Clone, PartialEq)]
pub struct Theme {
    pub kind: u8,
    pub surah: u8,
    pub from: u16,
    pub to: u16,
    pub text: String,
    pub number: usize,
    pub ruku: usize,
    pub start: f32,
    pub end: f32,
}

fn ruku_of(surah: u8, ayah: u16) -> usize {
    MUSHAF.ruku.iter().rposition(|r| (r.surah, r.ayah) <= (surah, ayah)).unwrap_or(0)
}

fn parse(json: &str) -> Option<Vec<Theme>> {
    let raw: Vec<(u8, u16, u16, u8, String)> = serde_json::from_str(json).ok()?;
    let (mut number, mut last) = (0, 0);
    Some(
        raw.into_iter()
            .map(|(surah, from, to, kind, text)| {
                number = if surah == last { number + 1 } else { 1 };
                last = surah;
                let s = &MUSHAF.surahs[surah as usize - 1];
                let end = if to >= s.ayat { s.end } else { ayah_pos((surah, to + 1)) };
                Theme { kind, surah, from, to, text, number, ruku: ruku_of(surah, from), start: ayah_pos((surah, from)), end }
            })
            .collect(),
    )
}

pub const NO_THEME: u8 = 2;
const CONTINUED: u8 = 1;

pub fn describe(t: &Theme) -> String {
    match t.kind {
        NO_THEME => format!("No theme · opens with {}", t.text),
        CONTINUED => format!("{} (continued)", t.text),
        _ => t.text.clone(),
    }
}

fn cells(themes: &[Theme], rel: Option<&Relations>) -> Vec<Cell> {
    themes
        .iter()
        .enumerate()
        .map(|(id, t)| {
            let r = &MUSHAF.ruku[t.ruku];
            let twins = rel.map_or(&[][..], |rel| rel.twins_of[id].as_slice());
            Cell {
                id,
                start: t.start,
                end: t.end,
                number: t.number,
                label: describe(t),
                tip: format!("{}:{}–{} · {}{}", t.surah, t.from, t.to, describe(t), if twins.is_empty() { String::new() } else { format!(" · ≈ {} look-alike", twins.len()) }),
                heavy: (r.surah, r.ayah) == (t.surah, t.from),
                level: twins.len().min(4) as u8,
            }
        })
        .collect()
}

#[derive(Clone)]
pub struct Themes(pub Rc<Vec<Theme>>);

impl PartialEq for Themes {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// State of the Ayah themes page beyond the chosen theme (Band::chosen). `intent` is the theme `open` chose last, so a
/// choice made on the band itself can drop a partner that belonged to another theme.
#[derive(Clone, Copy)]
pub struct View {
    pub themes: Signal<Option<Rc<Vec<Theme>>>>,
    pub with: Signal<Option<usize>>,
    pub twin: Signal<Option<usize>>,
    pub intent: Signal<Option<usize>>,
    pub revise: Signal<bool>,
    pub rel: Signal<Option<Rc<Relations>>>,
    pub rel_failed: Signal<bool>,
    pub data: Signal<Option<Rc<Data>>>,
    pub trail: Trail<(Option<usize>, Option<usize>)>,
}

/// Chooses theme `t` (and the comparison partner), showing its surah unless the selection already shows part of it.
pub fn open(view: View, band: Band, selection: Selection, themes: &[Theme], t: usize, with: Option<usize>) {
    let (mut chosen, mut partner, mut twin, mut intent, mut selected) = (band.chosen, view.with, view.twin, view.intent, selection.selected);
    let (start, end) = (themes[t].start, themes[t].end);
    if !(*selected.peek()).map(range_of).is_some_and(|(lo, hi)| start < hi && end > lo) {
        selected.set(Some(Focus::Surah(themes[t].surah as usize - 1)));
    }
    intent.set(Some(t));
    chosen.set(Some(t));
    partner.set(with);
    twin.set(None);
}

/// Opens look-alike `k` from `side` on the pair of themes the panel lists it under.
pub fn open_twin(view: View, band: Band, selection: Selection, themes: &[Theme], rel: &Relations, k: usize, side: usize) {
    if let Some((t, p)) = rel.twin_target(k, side) {
        open(view, band, selection, themes, t, p);
        let mut twin = view.twin;
        twin.set(Some(k));
    }
}

fn hash_theme(themes: &[Theme], t: Option<usize>) -> Option<String> {
    t.and_then(|t| themes.get(t)).map(|t| format!("{}:{}", t.surah, t.from))
}

fn theme_from_hash(themes: &[Theme], key: &str) -> Option<usize> {
    let (s, a) = quran_ui::hash::get(key).and_then(|v| quran_ui::hash::parse_ayah(&v))?;
    themes.iter().position(|t| t.surah == s && t.from <= a && a <= t.to)
}

#[component]
pub fn ThemesPage() -> Element {
    let band = use_context_provider(|| Band { cells: Signal::new(vec![]), hovered: Signal::new(None), chosen: Signal::new(None) });
    let overlay = use_context_provider(|| Overlay { dots: Signal::new(vec![]), marks: Signal::new(vec![]), picked: Signal::new(None), hot: Signal::new(vec![]) });
    let arcs = use_context_provider(|| Arcs { arcs: Signal::new(vec![]), ticks: Signal::new(vec![]), picked: Signal::new(None) });
    let view = use_context_provider(|| View {
        themes: Signal::new(None),
        with: Signal::new(None),
        twin: Signal::new(None),
        intent: Signal::new(None),
        revise: Signal::new(quran_ui::hash::get("view").as_deref() == Some("revise")),
        rel: Signal::new(None),
        rel_failed: Signal::new(false),
        data: Signal::new(None),
        trail: Trail::new(),
    });
    use_future(move || async move {
        let mut fetch = document::eval(FETCH);
        let Ok(text) = fetch.recv::<String>().await else { return };
        let Some(list) = parse(&text) else { return };
        let (mut chosen, mut with, mut intent, mut themes) = (band.chosen, view.with, view.intent, view.themes);
        let wanted = theme_from_hash(&list, "theme");
        intent.set(wanted);
        chosen.set(wanted);
        with.set(theme_from_hash(&list, "with").filter(|_| wanted.is_some()));
        let ranges: Vec<(u8, u16, u16)> = list.iter().map(|t| (t.surah, t.from, t.to)).collect();
        themes.set(Some(Rc::new(list)));
        let mut fetch = document::eval(FETCH_RELATIONS);
        let parsed = fetch.recv::<String>().await.ok().and_then(|json| Relations::parse(&json, &ranges));
        let (mut rel, mut failed) = (view.rel, view.rel_failed);
        failed.set(parsed.is_none());
        rel.set(parsed.map(Rc::new));
    });
    use_effect(move || {
        let (chosen, mut with, revise) = ((band.chosen)(), (view.with)(), (view.revise)());
        if chosen != *view.intent.peek() {
            let (mut with_signal, mut twin, mut intent) = (view.with, view.twin, view.intent);
            intent.set(chosen);
            twin.set(None);
            if with.take().is_some() {
                with_signal.set(None);
            }
        }
        if let Some(list) = (view.themes)() {
            quran_ui::hash::set(&[("theme", hash_theme(&list, chosen)), ("with", hash_theme(&list, with)), ("view", revise.then(|| "revise".to_string()))]);
            if chosen.is_some() {
                view.trail.record((chosen, with));
            }
        }
    });
    use_effect(move || {
        let (list, rel) = ((view.themes)(), (view.rel)());
        if let Some(list) = list {
            let mut cells_signal = band.cells;
            cells_signal.set(cells(&list, rel.as_deref()));
        }
    });
    use_lanes(view, band, overlay, arcs);

    rsx! {
        style { {include_str!("../assets/similar.css")} }
        style { {include_str!("../assets/themes.css")} }
        style { {include_str!("../assets/relations.css")} }
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
            extra: rsx! { Host {} },
        }
    }
}

/// Everything below the timeline; owns the effects that need the timeline's Selection.
#[component]
fn Host() -> Element {
    let selection = use_context::<Selection>();
    let (band, overlay, arcs, view) = (use_context::<Band>(), use_context::<Overlay>(), use_context::<Arcs>(), use_context::<View>());
    let mut similar_started = use_signal(|| false);
    use_effect(move || {
        let wanted = (band.chosen)().is_some() || (view.revise)() || (selection.selected)().is_some();
        let Some(rel) = (view.rel)() else { return };
        if !wanted || *similar_started.peek() {
            return;
        }
        similar_started.set(true);
        spawn(async move {
            for cache in ["default", "reload"] {
                let mut fetch = document::eval(&FETCH_SIMILAR.replace("CACHE", &format!("\"{cache}\"")));
                let Ok(json) = fetch.recv::<String>().await else { return };
                if let Some(d) = Data::parse(&json).filter(|d| (d.keys.len(), d.families.len()) == rel.version) {
                    let mut data = view.data;
                    data.set(Some(Rc::new(d)));
                    return;
                }
            }
        });
    });
    use_effect(move || {
        let (Some(key), Some(list), rel) = ((overlay.picked)(), (view.themes)(), (view.rel)()) else { return };
        let mut picked = overlay.picked;
        picked.set(None);
        let Some(t) = list.iter().position(|t| t.surah == key.0 && t.from <= key.1 && key.1 <= t.to) else { return };
        let (chosen, with) = (*band.chosen.peek(), *view.with.peek());
        let Some(c) = chosen else { return open(view, band, selection, &list, t, None) };
        // An ayah of the open comparison keeps it: on this passage's side stay, on the partner's side swap the two.
        let in_twin = rel.zip(index_of(key)).and_then(|(rel, g)| {
            rel.twins_for(c, with).into_iter().find_map(|(k, side)| {
                let s = rel.twins[k].sides;
                let inside = |r: (u16, u16)| r.0 <= g && g <= r.1;
                if inside(s[side]) { Some(true) } else if inside(s[1 - side]) { Some(false) } else { None }
            })
        });
        match (in_twin, with) {
            (Some(true), _) => {}
            (Some(false), Some(w)) => open(view, band, selection, &list, w, Some(c)),
            _ if t == c => {}
            _ if with == Some(t) => open(view, band, selection, &list, t, Some(c)),
            _ => open(view, band, selection, &list, t, None),
        }
    });
    use_effect(move || {
        let (Some((a, b, twin)), Some(list), Some(rel)) = ((arcs.picked)(), (view.themes)(), (view.rel)()) else { return };
        let mut picked = arcs.picked;
        picked.set(None);
        match twin {
            Some(k) => open_twin(view, band, selection, &list, &rel, k, 0),
            None => open(view, band, selection, &list, a, Some(b)),
        }
    });
    let Some(list) = (view.themes)() else { return rsx! { p { class: "themes-note", "Loading themes…" } } };
    let themes = Themes(list);
    let mut revise = view.revise;
    rsx! {
        if let Some(t) = (band.chosen)().filter(|&t| t < themes.0.len()) {
            ThemePanel { key: "{t}", themes: themes.clone(), t }
        }
        div { class: "tabs",
            button { class: if !revise() { "tab on" } else { "tab" }, onclick: move |_| revise.set(false), span { class: "ar", "الفهرس" } " Outline" }
            button { class: if revise() { "tab on" } else { "tab" }, onclick: move |_| revise.set(true), span { class: "ar", "راجِع معًا" } " Revise together" }
        }
        if revise() {
            Revise { themes: themes.clone() }
        } else {
            Outline { themes: themes.clone() }
        }
    }
}
