use crate::relations::{key_of, Seg, SharedRelations};
use crate::themes::View;
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{ayah_pos, Overlay, Selection};
use quran_ui::MUSHAF;

const STACK_WORDS: usize = 6;
const FORK_SHOWN: usize = 14;

fn mark(flag: u8, other: &str) -> (&'static str, Option<String>) {
    match flag {
        b'^' => ("w vow", Some(format!("Other vowels in the other passage: {other}"))),
        b'~' => ("w waw", Some(format!("The other passage has: {other}"))),
        _ => ("w", None),
    }
}

#[component]
fn Head(lo: u16, hi: u16, label: String, other: bool) -> Element {
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let key = key_of(lo);
    let page = ayah_pos(key).floor() as u32 + 1;
    rsx! {
        div { class: if other { "lad-col other" } else { "lad-col" },
            button {
                class: "ref",
                onclick: move |_| selection.show(key, overlay),
                oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(key.0, key.1)),
                span { class: "ar", "{MUSHAF.surahs[key.0 as usize - 1].name}" } " {label}"
            }
            span { class: "lad-page", "p {page}" }
        }
    }
}

/// One side of a fork. A long side shows FORK_SHOWN words: its end when it leads into shared wording (`tail`), else its start.
#[component]
fn Cell(words: Vec<String>, expanded: bool, tail: bool, onexpand: EventHandler<()>) -> Element {
    let long = words.len() > FORK_SHOWN && !expanded;
    let shown = match (long, tail) {
        (false, _) => &words[..],
        (true, true) => &words[words.len() - FORK_SHOWN..],
        (true, false) => &words[..FORK_SHOWN],
    };
    let text = if words.is_empty() { "—".to_string() } else { shown.join(" ") };
    let more = rsx! { button { class: if tail { "more lead" } else { "more" }, onclick: move |_| onexpand.call(()), "…" } };
    rsx! {
        div { class: "lad-cell", dir: "rtl",
            if long && tail { {more.clone()} }
            "{text}"
            if long && !tail { {more} }
        }
    }
}

#[component]
pub fn Ladder(rel: SharedRelations, k: usize, side: usize) -> Element {
    let view = use_context::<View>();
    let mut expanded = use_signal(Vec::<usize>::new);
    let data = (view.data)();
    let r = &rel.0;
    let twin = &r.twins[k];
    let (this, other) = (twin.sides[side], twin.sides[1 - side]);
    let Some(segs) = r.ladder(k, side, data.as_deref()) else { return rsx! { p { class: "themes-note", "Loading the passages…" } } };
    let then = (r.next_words(this.1, data.as_deref()), r.next_words(other.1, data.as_deref()));

    rsx! {
        div { class: "ladder", dir: "rtl",
            div { class: "lad-head",
                Head { lo: this.0, hi: this.1, label: r.range_label(this.0, this.1), other: false }
                span { class: "lad-mid", "≈" }
                Head { lo: other.0, hi: other.1, label: r.range_label(other.0, other.1), other: true }
            }
            for (n, seg) in segs.into_iter().enumerate() {
                match seg {
                    Seg::Same(words) => rsx! {
                        div { key: "{n}", class: "lad-same",
                            for (i, (w, f, other)) in words.into_iter().enumerate() {
                                span { key: "{i}", class: mark(f, &other).0, "data-tip": mark(f, &other).1, "{w} " }
                            }
                        }
                    },
                    Seg::Fork(a, b) => {
                        let long = a.len().max(b.len()) > STACK_WORDS;
                        let open = expanded.read().contains(&n);
                        rsx! {
                            div { key: "{n}", class: if long { "lad-fork long" } else { "lad-fork" },
                                Cell { words: a, expanded: open, tail: n == 0, onexpand: move |_| expanded.write().push(n) }
                                Cell { words: b, expanded: open, tail: n == 0, onexpand: move |_| expanded.write().push(n) }
                            }
                        }
                    }
                }
            }
            if then.0.is_some() || then.1.is_some() {
                div { class: "lad-fork then",
                    span { class: "lad-tag", span { class: "ar", "ثم" } " · then" }
                    Cell { words: then.0.clone().unwrap_or_default(), expanded: true, tail: false, onexpand: |_| {} }
                    Cell { words: then.1.clone().unwrap_or_default(), expanded: true, tail: false, onexpand: |_| {} }
                }
            }
        }
    }
}
