use crate::compare::{phrase_in, Compare};
use crate::ladder::Ladder;
use crate::relations::{key_of, Link, Relations, SharedRelations, ECHO, STORY, WORDING};
use crate::revise::RefrainChips;
use crate::themes::{describe, open, Theme, Themes, View};
use dioxus::prelude::*;
use quran_ui::band::Band;
use quran_ui::overlay::{Overlay, Selection};
use quran_ui::MUSHAF;

const ROWS_SHOWN: usize = 8;
const CHIP_WORDS: usize = 4;
const WORD_CHIPS: usize = 3;

struct Group {
    ar: &'static str,
    en: &'static str,
    rows: Vec<(Option<usize>, usize)>,
}

fn surah_name(themes: &[Theme], t: usize) -> &'static str {
    MUSHAF.surahs[themes[t].surah as usize - 1].name
}

/// The panel's sections; every partner appears once (Relations::listed).
fn groups(rel: &Relations, t: usize) -> (Vec<Group>, Vec<(Option<usize>, usize)>) {
    let mut g = [
        Group { ar: "مقطع متشابه", en: "look-alike passage", rows: vec![] },
        Group { ar: "قصة", en: "same story", rows: vec![] },
        Group { ar: "عبارة مشتركة", en: "shared wording", rows: vec![] },
        Group { ar: "موضوع", en: "same subject", rows: vec![] },
        Group { ar: "صدى في السورة", en: "echo in this surah", rows: vec![] },
    ];
    let mut weaker = vec![];
    for (l, p) in rel.listed(t) {
        let twin = !rel.twins_for(t, Some(p)).is_empty();
        let section = match l.map(|l| &rel.links[l]) {
            None => 0,
            Some(Link { kind: ECHO, .. }) => 4,
            Some(Link { strong: false, .. }) => {
                weaker.push((l, p));
                continue;
            }
            Some(_) if twin => 0,
            Some(Link { kind: STORY, .. }) => 1,
            Some(Link { kind: WORDING, .. }) => 2,
            Some(_) => 3,
        };
        g[section].rows.push((l, p));
    }
    for group in g.iter_mut() {
        group.rows.sort_by_key(|r| r.1);
    }
    weaker.sort_by_key(|r| r.1);
    (g.into_iter().filter(|g| !g.rows.is_empty()).collect(), weaker)
}

#[component]
pub fn ThemePanel(themes: Themes, t: usize) -> Element {
    let view = use_context::<View>();
    let band = use_context::<Band>();
    let selection = use_context::<Selection>();
    let list = themes.0.clone();
    let theme = &list[t];
    let rel = (view.rel)();
    let trail = view.trail;
    let (back_list, forward_list) = (list.clone(), list.clone());
    let go = move |delta: i32, list: &[Theme]| {
        if let Some((Some(c), with)) = trail.go(delta) {
            open(view, band, selection, list, c, with);
        }
    };
    let opening = rel.as_ref().map(|r| r.openings[t].clone()).unwrap_or_default();

    rsx! {
        div { class: "tpanel",
            div { class: "tp-head",
                div { class: "tp-title",
                    span { class: "tp-open", dir: "rtl", "{opening} …" }
                    span { class: "tp-ref", span { class: "ar", "{surah_name(&list, t)}" } " {theme.surah}:{theme.from}–{theme.to}" }
                }
                div { class: "tp-tools",
                    button { class: "nav", disabled: !trail.can(-1), "data-tip": "Back", onclick: move |_| go(-1, &back_list), "‹" }
                    button { class: "nav", disabled: !trail.can(1), "data-tip": "Forward", onclick: move |_| go(1, &forward_list), "›" }
                }
            }
            p { class: "tp-en", "{describe(theme)}" }
            match rel {
                None => rsx! { p { class: "themes-note", if (view.rel_failed)() { "Related passages are not available right now." } else { "Loading related passages…" } } },
                Some(rel) => {
                    let (sections, weaker) = groups(&rel, t);
                    let inside = rel.twins_for(t, None);
                    let empty = sections.is_empty() && weaker.is_empty() && inside.is_empty() && rel.refrains_of[t].is_empty();
                    rsx! {
                        if empty { p { class: "themes-note", "No related passages found for this theme." } }
                        for g in sections {
                            Section { key: "{g.en}", themes: themes.clone(), t, ar: g.ar, en: g.en, rows: g.rows, open_at_start: true }
                        }
                        if !weaker.is_empty() {
                            Section { themes: themes.clone(), t, ar: "أضعف", en: "weaker", rows: weaker, open_at_start: false }
                        }
                        if !inside.is_empty() {
                            div { class: "tp-sec",
                                p { class: "tp-sec-head", span { class: "ar", "داخل المقطع" } " · look-alikes inside this passage · {inside.len()}" }
                                for (k, side) in inside {
                                    Ladder { key: "{k}", rel: SharedRelations(rel.clone()), k, side }
                                }
                            }
                        }
                        if !rel.refrains_of[t].is_empty() {
                            div { class: "tp-sec",
                                p { class: "tp-sec-head", span { class: "ar", "لازمة" } " · refrain" }
                                RefrainChips { refrains: rel.refrains_of[t].clone() }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Section(themes: Themes, t: usize, ar: &'static str, en: &'static str, rows: Vec<(Option<usize>, usize)>, open_at_start: bool) -> Element {
    let view = use_context::<View>();
    let mut expanded = use_signal(|| open_at_start);
    let mut all = use_signal(|| false);
    let total = rows.len();
    let at = (view.with)().and_then(|p| rows.iter().position(|r| r.1 == p));
    use_effect(use_reactive!(|at| {
        if let Some(i) = at {
            expanded.set(true);
            if i >= ROWS_SHOWN {
                all.set(true);
            }
        }
    }));
    let shown = if all() { total } else { total.min(ROWS_SHOWN).max(at.map_or(0, |i| i + 1)) };
    let visible = expanded() || at.is_some();
    rsx! {
        div { class: "tp-sec",
            button { class: "tp-sec-head", onclick: move |_| expanded.toggle(),
                span { class: "ar", "{ar}" } " · {en} · {total}"
                if !open_at_start { span { class: "fold", if visible { " ▾" } else { " ▸" } } }
            }
            if visible {
                for (l, p) in rows.into_iter().take(shown) {
                    LinkRow { key: "{p}", themes: themes.clone(), t, link: l, p }
                }
                if shown < total {
                    button { class: "chips-more", onclick: move |_| all.set(true), "+{total - shown} more" }
                }
            }
        }
    }
}

#[component]
fn LinkRow(themes: Themes, t: usize, link: Option<usize>, p: usize) -> Element {
    let view = use_context::<View>();
    let band = use_context::<Band>();
    let selection = use_context::<Selection>();
    let overlay = use_context::<Overlay>();
    let Some(rel) = (view.rel)() else { return rsx! {} };
    let data = (view.data)();
    let list = themes.0.clone();
    let partner = &list[p];
    let is_open = (view.with)() == Some(p);
    let (mut with, mut focus) = (view.with, view.twin);
    let l = link.map(|l| &rel.links[l]);
    let dots = l.map_or(1, |l| 1 + (l.score as usize / 3).min(2));
    let twin = !rel.twins_for(t, Some(p)).is_empty();
    let phrases: Vec<String> = match (l, data.as_deref()) {
        (Some(l), Some(d)) => l.families.iter().filter(|&&f| f < d.families.len()).map(|&f| phrase_in(d, f, rel.spans[t], CHIP_WORDS)).collect(),
        (Some(l), None) => l.families.iter().map(|_| "…".to_string()).collect(),
        _ => vec![],
    };
    let people: Vec<String> = l.map_or(vec![], |l| l.concepts.iter().map(|&c| rel.concepts[c].0.clone()).collect());
    let words: Vec<String> = l.map_or(vec![], |l| {
        let (lo, hi) = rel.spans[t];
        l.words
            .iter()
            .take(WORD_CHIPS)
            .filter_map(|&(a, wa, b, wb)| { let (g, w) = if lo <= a && a <= hi { (a, wa) } else { (b, wb) }; rel.words(g, data.as_deref())?.get(w).cloned() })
            .collect()
    });
    let (lo, hi) = rel.spans[p];
    let keys: Vec<_> = (lo..=hi).map(key_of).collect();
    let go_list = list.clone();

    rsx! {
        div { class: if is_open { "lrow on" } else { "lrow" },
            div {
                class: "lmain",
                onmouseenter: move |_| {
                    let mut hot = overlay.hot;
                    hot.set(keys.clone());
                },
                onmouseleave: move |_| {
                    let mut hot = overlay.hot;
                    hot.set(vec![]);
                },
                onclick: move |_| {
                    focus.set(None);
                    with.set(if is_open { None } else { Some(p) });
                },
                button {
                    class: "ref lref",
                    "data-tip": "Go to {partner.surah}:{partner.from}",
                    onclick: move |e| {
                        e.stop_propagation();
                        open(view, band, selection, &go_list, p, Some(t));
                    },
                    span { class: "ar", "{surah_name(&list, p)}" }
                    " {partner.surah}:{partner.from}–{partner.to}"
                }
                span { class: "lopen", dir: "rtl", "{rel.openings[p]} …" }
                span { class: "ldots", "data-tip": "Strength", {"●".repeat(dots)} }
                span { class: "lchips", dir: "rtl",
                    if twin { span { class: "lchip twin", "data-tip": "Look-alike passage: open to compare word by word", "≈" } }
                    for (n, ph) in phrases.iter().enumerate() {
                        span { key: "p{n}", class: "lchip phrase", "«{ph}»" }
                    }
                    for (n, name) in people.iter().enumerate() {
                        span { key: "c{n}", class: "lchip person", "{name}" }
                    }
                    for (n, w) in words.iter().enumerate() {
                        span { key: "w{n}", class: "lchip word", "{w}" }
                    }
                }
            }
            p { class: "len", "{describe(partner)}" }
            if is_open {
                Compare { themes: themes.clone(), t, p }
            }
        }
    }
}
