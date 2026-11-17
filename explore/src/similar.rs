use dioxus::prelude::*;
use quran_ui::overlay::{ayah_pos, AyahKey, Overlay, Selection, Spot};
use quran_ui::MUSHAF;
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/similar-ayah";
const FETCH: &str = r#"fetch("similar.json").then(r => r.text()).then(t => dioxus.send(t))"#;

#[derive(Deserialize)]
#[serde(untagged)]
enum Entry {
    Family { p: String, m: Vec<(u8, u16, String, String)> },
    Variant { v: f32, m: Vec<(u8, u16, Vec<String>)> },
}

impl Entry {
    fn keys(&self) -> Vec<AyahKey> {
        match self {
            Entry::Family { m, .. } => m.iter().map(|r| (r.0, r.1)).collect(),
            Entry::Variant { m, .. } => m.iter().map(|r| (r.0, r.1)).collect(),
        }
    }
}

struct Data {
    entries: Vec<Entry>,
    by_ayah: HashMap<AyahKey, Vec<usize>>,
}

impl Data {
    fn parse(text: &str) -> Option<Data> {
        let entries: Vec<Entry> = serde_json::from_str(text).ok()?;
        let mut by_ayah: HashMap<AyahKey, Vec<usize>> = HashMap::new();
        for (i, e) in entries.iter().enumerate() {
            for k in e.keys() {
                by_ayah.entry(k).or_default().push(i);
            }
        }
        Some(Data { entries, by_ayah })
    }

    fn dots(&self) -> Vec<Spot> {
        self.by_ayah
            .iter()
            .map(|(&key, ids)| Spot {
                key,
                pos: ayah_pos(key),
                tip: format!("{}:{} · {}", key.0, key.1, plural(ids.len(), "similar group")),
                weight: ids.len().min(3) as u8,
            })
            .collect()
    }
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

fn label(key: AyahKey) -> String {
    format!("{}:{} · \u{2068}{}\u{2069}", key.0, key.1, MUSHAF.surahs[key.0 as usize - 1].name)
}

fn spots(keys: impl Iterator<Item = AyahKey>) -> Vec<Spot> {
    keys.map(|key| Spot { key, pos: ayah_pos(key), tip: label(key), weight: 3 }).collect()
}

#[component]
pub fn SimilarAyah() -> Element {
    let data = use_signal(|| None::<Rc<Data>>);
    let overlay = use_context_provider(|| Overlay { dots: Signal::new(vec![]), marks: Signal::new(vec![]), picked: Signal::new(None) });
    use_future(move || async move {
        let mut fetch = document::eval(FETCH);
        if let Ok(text) = fetch.recv::<String>().await {
            if let Some(d) = Data::parse(&text) {
                let mut dots = overlay.dots;
                dots.set(d.dots());
                let mut data = data;
                data.set(Some(Rc::new(d)));
            }
        }
    });

    let mut chosen = use_signal(|| None::<usize>);
    use_effect(move || {
        let _ = (overlay.picked)();
        chosen.set(None);
    });
    use_effect(move || {
        let keys = match (data(), (overlay.picked)()) {
            (Some(d), Some(key)) => {
                let ids = d.by_ayah.get(&key).cloned().unwrap_or_default();
                let mut keys: Vec<AyahKey> = match chosen() {
                    Some(i) if i < d.entries.len() => d.entries[i].keys(),
                    _ => ids.iter().flat_map(|&i| d.entries[i].keys()).collect(),
                };
                keys.sort();
                keys.dedup();
                keys
            }
            _ => vec![],
        };
        let mut marks = overlay.marks;
        marks.set(spots(keys.into_iter()));
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
            extra: rsx! { Groups { data: data().map(Shared), chosen } },
        }
    }
}

#[derive(Clone)]
struct Shared(Rc<Data>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[component]
fn Groups(data: Option<Shared>, mut chosen: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let picked = (overlay.picked)();

    let Some(Shared(data)) = data else { return rsx! { p { class: "groups-note", "Loading similar ayat…" } } };
    let Some(key) = picked else {
        return rsx! { p { class: "groups-note", "Select a surah or juz, then click a dot under the rub' row to see that ayah's similar ayat." } };
    };
    let mut ids = data.by_ayah.get(&key).cloned().unwrap_or_default();
    ids.sort_by_key(|&i| match &data.entries[i] {
        Entry::Family { p, m } => (0, usize::MAX - m.len(), usize::MAX - p.len()),
        Entry::Variant { .. } => (1, 0, 0),
    });

    let mut previous: Vec<AyahKey> = vec![];
    let rows: Vec<(usize, bool)> = ids
        .iter()
        .map(|&i| {
            let keys = data.entries[i].keys();
            let nested = !previous.is_empty() && keys.iter().all(|k| previous.contains(k));
            if !nested {
                previous = keys;
            }
            (i, nested)
        })
        .collect();

    rsx! {
        div { class: "groups",
            p { class: "groups-head", "{label(key)} · {plural(ids.len(), \"group\")}" }
            if ids.is_empty() { p { class: "groups-note", "No similar ayat found for this ayah." } }
            for (i, nested) in rows {
                div {
                    key: "{i}",
                    class: format!("group{}{}", if nested { " nested" } else { "" }, if chosen() == Some(i) { " on" } else { "" }),
                    onclick: move |_| chosen.set(if chosen() == Some(i) { None } else { Some(i) }),
                    match &data.entries[i] {
                        Entry::Family { p, m } => rsx! {
                            div { class: "group-head",
                                span { class: "phrase", dir: "rtl", "{p}" }
                                span { class: "count", "{m.len()} ayat" }
                            }
                            for (s, a, before, after) in m.iter().cloned() {
                                div {
                                    class: if (s, a) == key { "member self" } else { "member" },
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        selection.show((s, a), overlay);
                                    },
                                    span { class: "ref", "{label((s, a))}" }
                                    span { class: "cue", dir: "rtl",
                                        span { class: "before", "{before}" }
                                        span { class: "gap", " … " }
                                        span { class: "after", "{after}" }
                                    }
                                }
                            }
                        },
                        Entry::Variant { v, m } => rsx! {
                            div { class: "group-head",
                                span { class: "phrase", "Close variant" }
                                span { class: "count", if *v >= 1.0 { "differs only by و / ف" } else { "{(v * 100.0).round()}% alike" } }
                            }
                            for (s, a, words) in m.iter().cloned() {
                                div {
                                    class: if (s, a) == key { "member self" } else { "member" },
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        selection.show((s, a), overlay);
                                    },
                                    span { class: "ref", "{label((s, a))}" }
                                    span { class: "cue", dir: "rtl",
                                        for (n, w) in words.iter().enumerate() {
                                            span {
                                                key: "{n}",
                                                class: if w.starts_with('*') { "w diff" } else if w.starts_with('~') { "w pre" } else { "w" },
                                                "{w.trim_start_matches(['*', '~'])} "
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}
