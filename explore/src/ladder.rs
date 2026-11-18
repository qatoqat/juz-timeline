use crate::relations::{key_of, Seg, SharedRelations};
use crate::slips::twin_key;
use crate::themes::View;
use crate::tree::Drill;
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{ayah_pos, Overlay, Selection};
use quran_ui::MUSHAF;

const STACK_WORDS: usize = 6;
const FORK_SHOWN: usize = 14;
const THEN: usize = usize::MAX;

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
fn Cell(words: Vec<String>, hidden: bool, expanded: bool, tail: bool, onreveal: EventHandler<()>, onexpand: EventHandler<()>) -> Element {
    let long = words.len() > FORK_SHOWN && !expanded;
    let shown = match (long, tail) {
        (false, _) => &words[..],
        (true, true) => &words[words.len() - FORK_SHOWN..],
        (true, false) => &words[..FORK_SHOWN],
    };
    let text = if words.is_empty() { "—".to_string() } else { shown.join(" ") };
    let more = rsx! { button { class: if tail { "more lead" } else { "more" }, onclick: move |e| { e.stop_propagation(); onexpand.call(()) }, "…" } };
    rsx! {
        div { class: if hidden { "lad-cell hidden" } else { "lad-cell" }, dir: "rtl", onclick: move |_| onreveal.call(()),
            if long && tail && !hidden { {more.clone()} }
            "{text}"
            if long && !tail && !hidden { {more} }
        }
    }
}

#[component]
pub fn Ladder(rel: SharedRelations, k: usize, side: usize) -> Element {
    let view = use_context::<View>();
    let Drill(drill) = use_context::<Drill>();
    let mut shown = use_signal(Vec::<usize>::new);
    let mut expanded = use_signal(Vec::<usize>::new);
    let mut round = use_signal(|| 0usize);
    let mut answer = use_signal(|| None::<bool>);
    let mut slips = view.slips;
    let data = (view.data)();
    let r = &rel.0;
    let twin = &r.twins[k];
    let (this, other) = (twin.sides[side], twin.sides[1 - side]);
    let Some(segs) = r.ladder(k, side, data.as_deref()) else { return rsx! { p { class: "themes-note", "Loading the passages…" } } };
    let then = (r.next_words(this.1, data.as_deref()), r.next_words(other.1, data.as_deref()));
    let key = twin_key(twin);
    let slipped = slips.read().has(&key);
    let same_surah = key_of(this.0).0 == key_of(other.0).0;
    let (this_label, other_label) = (r.range_label(this.0, this.1), r.range_label(other.0, other.1));
    let quiz: Vec<(String, String)> = segs
        .iter()
        .filter_map(|s| match s {
            Seg::Fork(a, b) if !a.is_empty() && !b.is_empty() => Some((a.join(" "), b.join(" "))),
            _ => None,
        })
        .collect();
    let hide = |n: usize| drill() && !shown.read().contains(&n);
    let asked = (!quiz.is_empty()).then(|| {
        let n = round() % quiz.len();
        let ask_this = (round() / quiz.len() + n) % 2 == 0;
        (if ask_this { quiz[n].0.clone() } else { quiz[n].1.clone() }, ask_this)
    });
    let names = if same_surah {
        (this_label.clone(), other_label.clone())
    } else {
        (MUSHAF.surahs[key_of(this.0).0 as usize - 1].name.to_string(), MUSHAF.surahs[key_of(other.0).0 as usize - 1].name.to_string())
    };
    let answer_key = key.clone();
    let quiz_key = key.clone();

    rsx! {
        div { class: if slipped { "ladder slipped" } else { "ladder" }, dir: "rtl",
            div { class: "lad-head",
                Head { lo: this.0, hi: this.1, label: this_label.clone(), other: false }
                span { class: "lad-mid", "≈" }
                Head { lo: other.0, hi: other.1, label: other_label.clone(), other: true }
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
                                Cell { words: a, hidden: hide(n), expanded: open, tail: n == 0, onreveal: move |_| shown.write().push(n), onexpand: move |_| expanded.write().push(n) }
                                Cell { words: b, hidden: hide(n), expanded: open, tail: n == 0, onreveal: move |_| shown.write().push(n), onexpand: move |_| expanded.write().push(n) }
                            }
                        }
                    }
                }
            }
            if then.0.is_some() || then.1.is_some() {
                div { class: "lad-fork then",
                    span { class: "lad-tag", span { class: "ar", "ثم" } " · then" }
                    Cell { words: then.0.clone().unwrap_or_default(), hidden: hide(THEN), expanded: true, tail: false, onreveal: move |_| shown.write().push(THEN), onexpand: |_| {} }
                    Cell { words: then.1.clone().unwrap_or_default(), hidden: hide(THEN), expanded: true, tail: false, onreveal: move |_| shown.write().push(THEN), onexpand: |_| {} }
                }
            }
            if drill() {
                div { class: "lad-drill", dir: "ltr",
                    span { class: "hint", "Recite, tap each hidden cell to check, then:" }
                    button { class: "chip right", onclick: move |_| { slips.write().answer(answer_key.clone(), true); shown.set(vec![]); }, span { class: "ar", "أتقنتُ" } " got it" }
                    button { class: "chip wrong", onclick: move |_| { slips.write().answer(key.clone(), false); shown.set(vec![]); }, span { class: "ar", "زللتُ" } " slipped" }
                }
                if let Some((text, ask_this)) = asked {
                    div { class: "quiz lad-quiz", dir: "ltr",
                        p { class: "quiz-q",
                            span { class: "ar", if same_surah { "أيّ موضع؟" } else { "أيّ سورة؟" } }
                            if same_surah { " Which place says it this way?" } else { " Which surah says it this way?" }
                        }
                        p { class: "phrase", dir: "rtl", "{text}" }
                        div { class: "quiz-choices",
                            for (n, (label, is_this)) in [(names.0.clone(), true), (names.1.clone(), false)].into_iter().enumerate() {
                                {
                                    let quiz_key = quiz_key.clone();
                                    let state = answer().map(|right| if is_this == ask_this { "chip right" } else if right { "chip" } else { "chip wrong" });
                                    rsx! {
                                        button {
                                            key: "{n}",
                                            class: state.unwrap_or("chip"),
                                            disabled: answer().is_some(),
                                            onclick: move |_| {
                                                let right = is_this == ask_this;
                                                answer.set(Some(right));
                                                slips.write().answer(quiz_key.clone(), right);
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
                        }
                        if answer().is_some() {
                            p { class: "quiz-a", button { class: "nav", onclick: move |_| { round += 1; answer.set(None); }, "Next ›" } }
                        }
                    }
                }
            }
        }
    }
}
