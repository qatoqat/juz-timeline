use crate::card::Card;
use crate::data::Data;
use crate::ladder::Ladder;
use crate::relations::{index_of, key_of, SharedRelations};
use crate::themes::{describe, Themes, View};
use crate::tree::{Drill, Shared};
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{Overlay, Selection};
use quran_ui::MUSHAF;
use std::rc::Rc;

const CONTEXT_WORDS: usize = 6;

/// The member of a phrase family that lies in the ayah range, or its first member.
pub fn member_in(d: &Data, family: usize, span: (u16, u16)) -> usize {
    let members = &d.families[family].members;
    members.iter().map(|m| m.0).find(|&a| index_of(d.keys[a]).is_some_and(|g| span.0 <= g && g <= span.1)).unwrap_or(members[0].0)
}

/// The phrase as written in the range's own ayah, shortened to about `max` words.
pub fn phrase_in(d: &Data, family: usize, span: (u16, u16), max: usize) -> String {
    let a = member_in(d, family, span);
    let (start, end) = d.span_in(family, a);
    let words = d.words(a);
    let w = &words[start.min(words.len())..end.min(words.len())];
    if w.len() <= max + 2 { w.join(" ") } else { format!("{} … {}", w[..max - 1].join(" "), w[w.len() - 2..].join(" ")) }
}

#[component]
pub fn Compare(themes: Themes, t: usize, p: usize) -> Element {
    let view = use_context::<View>();
    let Drill(drill) = use_context::<Drill>();
    let mut evidence = use_signal(|| false);
    let Some(rel) = (view.rel)() else { return rsx! {} };
    let data = (view.data)();
    let link = rel.links_of[t].iter().map(|&l| &rel.links[l]).find(|l| l.i == p || l.j == p);
    let mut twins = rel.twins_for(t, Some(p));
    let focus = (view.twin)();
    twins.sort_by_key(|&(k, _)| Some(k) != focus);
    let families: Vec<usize> = link.map_or(vec![], |l| l.families.clone());
    let words: Vec<(u16, usize, u16, usize)> = link.map_or(vec![], |l| {
        l.words.iter().map(|&(a, wa, b, wb)| if rel.spans[t].0 <= a && a <= rel.spans[t].1 { (a, wa, b, wb) } else { (b, wb, a, wa) }).collect()
    });
    let people: Vec<String> = link.map_or(vec![], |l| l.concepts.iter().map(|&c| format!("{} · {}", rel.concepts[c].0, rel.concepts[c].1)).collect());
    let only_people = twins.is_empty() && families.is_empty() && words.is_empty();
    let spoils = drill() && !evidence() && !(families.is_empty() && words.is_empty());
    let shared_rel = SharedRelations(rel.clone());
    let list = themes.0.clone();

    rsx! {
        div { class: "compare",
            for (k, side) in twins {
                Ladder { key: "{k}", rel: shared_rel.clone(), k, side }
            }
            if spoils {
                button { class: "chips-more", onclick: move |_| evidence.set(true), "Show the shared phrases (they reveal the answers)" }
            }
            if !families.is_empty() && !spoils {
                div { class: "cmp-sec",
                    p { class: "cmp-head", span { class: "ar", "عبارات مشتركة" } " · shared phrases" }
                    match data.clone() {
                        None => rsx! { p { class: "themes-note", "Loading phrases…" } },
                        Some(d) => rsx! {
                            for f in families.into_iter().filter(|&f| f < d.families.len()) {
                                {
                                    let picked = member_in(&d, f, rel.spans[t]);
                                    let (s, a) = d.keys[picked];
                                    let (start, end) = d.span_in(f, picked);
                                    rsx! {
                                        Card { key: "f{f}", data: Shared(d.clone()), family: f, picked }
                                        a { class: "tree-link", href: "../similar-ayah/#surah={s}&ayah={s}:{a}&phrase={start}:{end - start}", "Open the word tree ↗" }
                                    }
                                }
                            }
                        },
                    }
                }
            }
            if !words.is_empty() && !spoils {
                div { class: "cmp-sec",
                    p { class: "cmp-head", span { class: "ar", "كلمات نادرة مشتركة" } " · shared rare words" }
                    for (n, (a, wa, b, wb)) in words.into_iter().enumerate() {
                        div { key: "{n}", class: "snip-pair",
                            Snippet { rel: shared_rel.clone(), data: data.clone().map(Shared), g: a, w: wa }
                            Snippet { rel: shared_rel.clone(), data: data.clone().map(Shared), g: b, w: wb }
                        }
                    }
                }
            }
            if only_people {
                div { class: "cmp-sec",
                    p { class: "cmp-head", span { class: "ar", "قصة واحدة" } " · the same people" }
                    if !people.is_empty() { p { class: "cmp-people", "{people.join(\" · \")}" } }
                    div { class: "cmp-heads",
                        p { "{describe(&list[t])}" }
                        p { "{describe(&list[p])}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Snippet(rel: SharedRelations, data: Option<Shared>, g: u16, w: usize) -> Element {
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let data: Option<Rc<Data>> = data.map(|d| d.0);
    let key = key_of(g);
    let surah = MUSHAF.surahs[key.0 as usize - 1].name;
    let Some(words) = rel.0.words(g, data.as_deref()) else { return rsx! { p { class: "themes-note", "…" } } };
    let lo = w.saturating_sub(CONTEXT_WORDS);
    let hi = (w + CONTEXT_WORDS + 1).min(words.len());
    rsx! {
        div { class: "snip",
            button {
                class: "ref",
                onclick: move |_| selection.show(key, overlay),
                oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(key.0, key.1)),
                "{key.0}:{key.1} · {surah}"
            }
            span { class: "snip-text", dir: "rtl",
                if lo > 0 { span { class: "ell", "… " } }
                for (i, word) in words[lo.min(hi)..hi].iter().enumerate() {
                    span { key: "{i}", class: if lo + i == w { "w cue" } else { "w" }, "{word} " }
                }
                if hi < words.len() { span { class: "ell", "…" } }
            }
        }
    }
}
