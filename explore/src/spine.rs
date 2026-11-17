use crate::tree::{ayah_label, light, surah_name, unlight, Drill, Shared};
use dioxus::prelude::*;
use quran_ui::overlay::{Overlay, Selection};

const SPINE_PAD: usize = 2;
const DRILL_KEY: &str = "quran-timeline:drill";

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn load_drill() -> bool {
    storage().and_then(|s| s.get_item(DRILL_KEY).ok()?).as_deref() == Some("1")
}

fn save_drill(on: bool) {
    if let Some(s) = storage() {
        let _ = s.set_item(DRILL_KEY, if on { "1" } else { "0" });
    }
}

#[component]
pub fn Spine(data: Shared, ayah: usize, root: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let Drill(mut drill) = use_context::<Drill>();
    let d = data.0.clone();
    let words = d.words(ayah);
    let current = root();
    let mut spans: Vec<(usize, usize, usize)> = d.family_of[ayah]
        .iter()
        .filter(|&&f| current.is_none_or(|r| !d.folded(f, r, ayah)))
        .map(|&f| {
            let (s, e) = d.span_in(f, ayah);
            (f, s, e)
        })
        .collect();
    spans.sort_by_key(|&(_, s, e)| (s, usize::MAX - e));
    let mut ends: Vec<usize> = vec![];
    let bars: Vec<(usize, usize, usize, usize)> = spans
        .into_iter()
        .map(|(f, s, e)| {
            let layer = ends.iter().position(|&end| end <= s).unwrap_or_else(|| {
                ends.push(0);
                ends.len() - 1
            });
            ends[layer] = e;
            (f, s, e, layer)
        })
        .collect();
    let lo = bars.iter().map(|b| b.1).min().unwrap_or(0).saturating_sub(SPINE_PAD);
    let hi = (bars.iter().map(|b| b.2).max().unwrap_or(words.len()) + SPINE_PAD).min(words.len());
    let shown: Vec<(usize, String)> = (lo..hi).map(|i| (i, words[i].to_string())).collect();
    let total = words.len();

    rsx! {
        div { class: "spine-wrap",
            div { class: "spine-head",
                span { "{ayah_label(&d, ayah)} · " span { class: "ar", "{surah_name(&d, ayah)}" } " · {plural(d.family_of[ayah].len(), \"shared phrase\")}" }
                if bars.len() > 1 { span { class: "hint", " · tap a bar to branch from that phrase" } }
                button {
                    class: if drill() { "drill on" } else { "drill" },
                    "data-tip": "Self-test: hide what follows the phrase, tap to reveal",
                    onclick: move |_| {
                        let next = !drill();
                        drill.set(next);
                        save_drill(next);
                    },
                    "اختبار"
                }
            }
            div { class: "spine", dir: "rtl",
                if lo > 0 { span { class: "ellipsis", "…" } }
                div { class: "spine-grid", style: "grid-template-columns: repeat({hi - lo}, auto)",
                    for (i, w) in shown {
                        span { key: "w{i}", class: "sw", style: "grid-row:1;grid-column:{i - lo + 1}", "{w}" }
                    }
                    for (f, s, e, layer) in bars {
                        div {
                            key: "f{f}",
                            class: if current == Some(f) { "sbar on" } else { "sbar" },
                            style: "grid-row:{layer + 2};grid-column:{s - lo + 1} / {e - lo + 1}",
                            "data-tip": "{d.phrase(f)} · {plural(d.families[f].members.len(), \"ayah\")}",
                            onclick: {
                                let d = d.clone();
                                move |_| {
                                    root.set(Some(f));
                                    let ayat: Vec<usize> = d.families[f].members.iter().map(|m| m.0).collect();
                                    light(overlay, &d, &ayat);
                                }
                            },
                            onmouseenter: {
                                let d = d.clone();
                                move |_| {
                                    let ayat: Vec<usize> = d.families[f].members.iter().map(|m| m.0).collect();
                                    light(overlay, &d, &ayat);
                                }
                            },
                            onmouseleave: move |_| unlight(overlay),
                        }
                    }
                }
                if hi < total { span { class: "ellipsis", "…" } }
            }
        }
    }
}

#[component]
pub fn Variants(data: Shared, ayah: usize) -> Element {
    let selection = use_context::<Selection>();
    let overlay = use_context::<Overlay>();
    let d = data.0.clone();
    let list = d.variant_of[ayah].clone();
    if list.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "variants",
            p { class: "variants-head", "Close variants" }
            for v in list {
                div { key: "{v}", class: "variant",
                    span { class: "count",
                        if d.variants[v].ratio >= 1.0 { "differs only by و / ف" } else { "{(d.variants[v].ratio * 100.0).round()}% alike" }
                    }
                    for (a, flags) in d.variants[v].members.clone() {
                        div {
                            key: "{a}",
                            class: if a == ayah { "member self" } else { "member" },
                            onclick: {
                                let key = d.keys[a];
                                move |_| selection.show(key, overlay)
                            },
                            span { class: "ref", "{ayah_label(&d, a)} · {surah_name(&d, a)}" }
                            span { class: "cue", dir: "rtl",
                                for (n, (w, f)) in d.words(a).into_iter().zip(flags.chars()).enumerate() {
                                    span { key: "{n}", class: match f { '*' => "w diff", '~' => "w pre", _ => "w" }, "{w} " }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
