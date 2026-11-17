use crate::data::Data;
use crate::tree::{ayah_label, surah_name, Shared, Tree};
use dioxus::prelude::*;
use quran_ui::overlay::{ayah_pos, AyahKey, Overlay, Selection, Spot};
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/similar-ayah";
const FETCH: &str = r#"fetch("similar.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const SPINE_PAD: usize = 2;

fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

fn dots(data: &Data) -> Vec<Spot> {
    (0..data.keys.len())
        .map(|a| {
            let n = data.family_of[a].len() + data.variant_of[a].len();
            let key = data.keys[a];
            Spot { key, pos: ayah_pos(key), tip: format!("{}:{} · {}", key.0, key.1, plural(n, "similar group")), weight: n.min(3) as u8 }
        })
        .collect()
}

fn related(data: &Data, a: usize, root: Option<usize>) -> Vec<AyahKey> {
    let mut keys: Vec<AyahKey> = root.map(|f| data.families[f].members.iter().map(|m| data.keys[m.0]).collect()).unwrap_or_default();
    for &v in &data.variant_of[a] {
        keys.extend(data.variants[v].members.iter().map(|m| data.keys[m.0]));
    }
    keys.push(data.keys[a]);
    keys.sort();
    keys.dedup();
    keys
}

#[component]
pub fn SimilarAyah() -> Element {
    let mut data = use_signal(|| None::<Rc<Data>>);
    let overlay = use_context_provider(|| Overlay {
        dots: Signal::new(vec![]),
        marks: Signal::new(vec![]),
        picked: Signal::new(None),
        hot: Signal::new(vec![]),
    });
    let mut root = use_signal(|| None::<usize>);
    use_future(move || async move {
        let mut fetch = document::eval(FETCH);
        if let Ok(text) = fetch.recv::<String>().await {
            if let Some(d) = Data::parse(&text) {
                let mut dots_signal = overlay.dots;
                dots_signal.set(dots(&d));
                data.set(Some(Rc::new(d)));
            }
        }
    });
    use_effect(move || {
        let ayah = data().zip((overlay.picked)()).and_then(|(d, key)| d.slot.get(&key).and_then(|&a| d.default_root(a)));
        root.set(ayah);
    });
    use_effect(move || {
        let keys = match (data(), (overlay.picked)()) {
            (Some(d), Some(key)) => d.slot.get(&key).map(|&a| related(&d, a, root())).unwrap_or_default(),
            _ => vec![],
        };
        let mut marks = overlay.marks;
        marks.set(keys.into_iter().map(|key| Spot { key, pos: ayah_pos(key), tip: format!("{}:{}", key.0, key.1), weight: 3 }).collect());
    });

    rsx! {
        style { {include_str!("../assets/similar.css")} }
        quran_ui::Page {
            ruku: true,
            title: rsx! {
                h1 {
                    "آيات متشابهة"
                    span { class: "sub", "Similar ayah" }
                    a { class: "explore", href: "../", "Explore" }
                    a { class: "explore", href: "{QUL}", target: "_blank", rel: "noopener", "Source ↗" }
                }
            },
            extra: rsx! { Panel { data: data().map(Shared), root } },
        }
    }
}

#[component]
fn Panel(data: Option<Shared>, root: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let Some(shared) = data else { return rsx! { p { class: "groups-note", "Loading similar ayat…" } } };
    let Some(key) = (overlay.picked)() else {
        return rsx! { p { class: "groups-note", "Select a surah or juz, then click a dot under the rub' row to see that ayah's similar ayat." } };
    };
    let d = shared.0.clone();
    let Some(&a) = d.slot.get(&key) else {
        return rsx! { p { class: "groups-note", "{key.0}:{key.1} has no similar ayat in this data." } };
    };

    rsx! {
        div { class: "similar",
            Spine { data: shared.clone(), ayah: a, root }
            if let Some(f) = root() {
                Tree { key: "{f}-{a}", data: shared.clone(), family: f, picked: a }
            }
            Variants { data: shared, ayah: a }
        }
    }
}

#[component]
fn Spine(data: Shared, ayah: usize, root: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let d = data.0.clone();
    let words = &d.text[ayah];
    let mut spans: Vec<(usize, usize, usize)> = d.family_of[ayah]
        .iter()
        .map(|&f| {
            let s = d.start_in(f, ayah);
            (f, s, s + d.families[f].len)
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
    let shown: Vec<(usize, String)> = (lo..hi).map(|i| (i, words[i].clone())).collect();

    rsx! {
        div { class: "spine-wrap",
            div { class: "spine-head",
                "{ayah_label(&d, ayah)} · {surah_name(&d, ayah)} · "
                {plural(d.family_of[ayah].len(), "shared phrase")}
                if !bars.is_empty() { span { class: "hint", " · tap a bar to branch from that phrase" } }
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
                            class: if root() == Some(f) { "sbar on" } else { "sbar" },
                            style: "grid-row:{layer + 2};grid-column:{s - lo + 1} / {e - lo + 1}",
                            "data-tip": "{d.phrase(f)} · {plural(d.families[f].members.len(), \"ayah\")}",
                            onclick: move |_| root.set(Some(f)),
                            onmouseenter: {
                                let d = d.clone();
                                move |_| {
                                    let mut hot = overlay.hot;
                                    hot.set(d.families[f].members.iter().map(|m| d.keys[m.0]).collect());
                                }
                            },
                            onmouseleave: move |_| {
                                let mut hot = overlay.hot;
                                hot.set(vec![]);
                            },
                        }
                    }
                }
                if hi < words.len() { span { class: "ellipsis", "…" } }
            }
        }
    }
}

#[component]
fn Variants(data: Shared, ayah: usize) -> Element {
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
                                for (n, (w, f)) in d.text[a].iter().zip(flags.chars()).enumerate() {
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
