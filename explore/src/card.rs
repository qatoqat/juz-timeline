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
    let phrase = d.phrase(family);

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

fn order(n: usize, seed: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..n).collect();
    let mut state = (seed as u64).wrapping_mul(2_654_435_761).wrapping_add(1);
    for i in (1..n).rev() {
        state = state.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        idx.swap(i, ((state >> 33) % (i as u64 + 1)) as usize);
    }
    idx
}

#[component]
pub fn Quiz(data: Shared, family: usize) -> Element {
    let mut question = use_signal(|| 0usize);
    let mut answer = use_signal(|| None::<usize>);
    let d = data.0.clone();
    let rows = d.rows(family);
    let n = rows.len();
    let turn = question();
    let asked = &rows[order(n, family + turn / n)[turn % n]];
    let prompt = if asked.after.is_empty() { "۝".to_string() } else { asked.after.join(" ") };
    let choices = order(n, family * 7 + turn);
    let correct = asked.ayah;

    rsx! {
        div { class: "quiz",
            p { class: "quiz-q",
                "Which ayah continues this way? "
                span { class: "phrase", dir: "rtl", "{d.phrase(family)} " span { class: "w cue", "{prompt}" } }
            }
            div { class: "quiz-choices",
                for c in choices {
                    {
                        let a = rows[c].ayah;
                        let state = match answer() {
                            Some(_) if a == correct => " right",
                            Some(x) if x == a => " wrong",
                            _ => "",
                        };
                        rsx! {
                            button {
                                key: "{a}",
                                class: "chip{state}",
                                disabled: answer().is_some(),
                                onclick: move |_| answer.set(Some(a)),
                                "{ayah_label(&d, a)} · {surah_name(&d, a)}"
                            }
                        }
                    }
                }
            }
            if let Some(x) = answer() {
                p { class: "quiz-a",
                    if x == correct { "Correct." } else { "Not quite — it is {ayah_label(&d, correct)} · {surah_name(&d, correct)}." }
                    button {
                        class: "nav",
                        onclick: move |_| {
                            answer.set(None);
                            question += 1;
                        },
                        "Next ›"
                    }
                }
            }
        }
    }
}
