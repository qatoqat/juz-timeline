use crate::data::Data;
use crate::nav::{Nav, NavBar};
use crate::spine::{load_drill, plural, Spine, Variants};
use crate::tree::{Drill, Shared, Tree};
use dioxus::prelude::*;
use quran_ui::overlay::{ayah_pos, range_of, AyahKey, Overlay, Selection, Spot};
use quran_ui::MUSHAF;
use std::collections::HashMap;
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/similar-ayah";
const FETCH: &str = r#"fetch("similar.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const SEARCH_LIMIT: usize = 12;
const OVERVIEW_FIRST: usize = 12;
const OVERVIEW_MAX: usize = 40;
const CHIP_WORDS: usize = 4;

fn dots(data: &Data) -> Vec<Spot> {
    let m = &*MUSHAF;
    let rukus: std::collections::HashSet<AyahKey> = m.ruku.iter().filter(|r| r.ayah != 1).map(|r| (r.surah, r.ayah)).collect();
    let sajdahs: HashMap<AyahKey, usize> = m.sajdah.iter().enumerate().map(|(i, sj)| ((sj.surah, sj.ayah), i + 1)).collect();
    let mut spots = vec![];
    for surah in &m.surahs {
        for ayah in 1..=surah.ayat {
            let key = (surah.num as u8, ayah);
            let n = data.slot.get(&key).map_or(0, |&a| data.family_of[a].len() + data.variant_of[a].len());
            let ruku = rukus.contains(&key).then_some(());
            let sajdah = sajdahs.get(&key).copied();
            let mut tip = format!("{}:{} · {}", key.0, key.1, if n == 0 { "no similar ayat".to_string() } else { plural(n, "similar group") });
            if ruku.is_some() {
                tip += " · ruku' starts";
            }
            if let Some(sj) = sajdah {
                tip += &format!(" · sajdah {sj}");
            }
            spots.push(Spot { key, pos: ayah_pos(key), tip, weight: n.min(3) as u8, ruku: ruku.is_some(), sajdah: sajdah.is_some() });
        }
    }
    spots
}

fn related(data: &Data, a: usize, root: Option<usize>) -> Vec<AyahKey> {
    let mut ayat: Vec<usize> = vec![a];
    if let Some(f) = root {
        ayat.extend(data.families[f].members.iter().map(|m| m.0));
        ayat.extend(data.late_ayat(f, a));
    }
    for &v in &data.variant_of[a] {
        ayat.extend(data.variants[v].members.iter().map(|m| m.0));
    }
    let mut keys: Vec<AyahKey> = ayat.into_iter().map(|i| data.keys[i]).collect();
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
    use_context_provider(|| Drill(Signal::new(load_drill())));
    let mut root = use_signal(|| None::<usize>);
    let mut pending = use_signal(|| None::<usize>);
    let nav = use_context_provider(|| Nav::new(root, pending));
    use_effect(move || {
        if let (Some(f), Some(key)) = (root(), *overlay.picked.peek()) {
            nav.record(key, f);
        }
    });
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
        let wanted = *pending.peek();
        let next = data().zip((overlay.picked)()).and_then(|(d, key)| {
            let a = *d.slot.get(&key)?;
            wanted.filter(|f| d.family_of[a].contains(f)).or_else(|| d.default_root(a))
        });
        pending.set(None);
        root.set(next);
    });
    use_effect(move || {
        let keys = match (data(), (overlay.picked)()) {
            (Some(d), Some(key)) => d.slot.get(&key).map(|&a| related(&d, a, root())).unwrap_or_default(),
            _ => vec![],
        };
        let mut marks = overlay.marks;
        marks.set(keys.into_iter().map(|key| Spot { key, pos: ayah_pos(key), tip: format!("{}:{}", key.0, key.1), weight: 3, ..Spot::default() }).collect());
    });

    rsx! {
        style { {include_str!("../assets/similar.css")} }
        quran_ui::Page {
            ruku: false,
            title: rsx! {
                h1 {
                    "آيات متشابهة"
                    span { class: "sub", "Similar ayah" }
                    a { class: "explore", href: "../", "Explore" }
                    a { class: "explore", href: "{QUL}", target: "_blank", rel: "noopener", "Source ↗" }
                }
            },
            extra: rsx! { Panel { data: data().map(Shared), root, pending } },
        }
    }
}

#[component]
fn Chips(data: Shared, title: Option<String>, families: Vec<usize>, pick: EventHandler<usize>) -> Element {
    let d = data.0.clone();
    rsx! {
        if let Some(t) = title { p { class: "chips-head", "{t}" } }
        div { class: "chips", dir: "rtl",
            for f in families {
                button {
                    key: "{f}",
                    class: "chip",
                    "data-tip": "{d.phrase(f)} · {plural(d.families[f].members.len(), \"ayah\")}",
                    onclick: move |_| pick.call(f),
                    "{d.short_phrase(f, CHIP_WORDS)}"
                    span { class: "badge", "×{d.families[f].members.len()}" }
                }
            }
        }
    }
}

#[component]
fn Panel(data: Option<Shared>, root: Signal<Option<usize>>, pending: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let mut query = use_signal(String::new);
    let mut all = use_signal(|| false);
    let Some(shared) = data else { return rsx! { p { class: "groups-note", "Loading similar ayat…" } } };
    let d = shared.0.clone();
    let range = (selection.selected)().map(range_of);
    let picked = (overlay.picked)().and_then(|key| d.slot.get(&key).copied());
    let searching = !query().trim().is_empty();
    let results = d.search(&query(), SEARCH_LIMIT);
    let (inside, elsewhere) = match (picked, range) {
        (None, Some((lo, hi))) if !searching => d.in_range(lo, hi),
        _ => (vec![], vec![]),
    };
    let limit = if all() { OVERVIEW_MAX } else { OVERVIEW_FIRST };
    let more = inside.len() + elsewhere.len() > limit;
    let inside_shown: Vec<usize> = inside.iter().copied().take(limit).collect();
    let elsewhere_shown: Vec<usize> = elsewhere.iter().copied().take(limit.saturating_sub(inside_shown.len())).collect();

    let pick_d = d.clone();
    let pick = move |f: usize| {
        let inside = range.and_then(|(lo, hi)| pick_d.families[f].members.iter().find(|m| pick_d.pos[m.0] >= lo && pick_d.pos[m.0] < hi));
        let (a, _) = *inside.unwrap_or(&pick_d.families[f].members[0]);
        pending.set(Some(f));
        query.set(String::new());
        if inside.is_some() {
            let mut picked = overlay.picked;
            picked.set(Some(pick_d.keys[a]));
        } else {
            selection.show(pick_d.keys[a], overlay);
        }
    };

    rsx! {
        div { class: "similar",
            div { class: "finder",
                input {
                    class: "phrase-search",
                    r#type: "search",
                    dir: "rtl",
                    placeholder: "ابحث عن عبارة · search a phrase",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
                if searching && results.is_empty() { span { class: "groups-note", "No shared phrase matches." } }
            }
            if searching && !results.is_empty() {
                Chips { data: shared.clone(), title: None, families: results, pick: pick.clone() }
            }
            if !inside_shown.is_empty() {
                Chips { data: shared.clone(), title: Some("Repeated inside this selection".to_string()), families: inside_shown, pick: pick.clone() }
            }
            if !elsewhere_shown.is_empty() {
                Chips { data: shared.clone(), title: Some("Shared with other surahs".to_string()), families: elsewhere_shown, pick }
            }
            if more && !searching && picked.is_none() {
                button { class: "chips-more", onclick: move |_| all.toggle(), if all() { "Show fewer" } else { "Show more" } }
            }
            match picked {
                Some(a) => rsx! {
                    NavBar {}
                    Spine { data: shared.clone(), ayah: a, root }
                    if let Some(f) = root() {
                        Tree { key: "{f}-{a}", data: shared.clone(), family: f, picked: a }
                    }
                    Variants { data: shared.clone(), ayah: a }
                },
                None if (overlay.picked)().is_some() => rsx! { p { class: "groups-note", "This ayah has no similar ayat in this data." } },
                None if range.is_none() && !searching => rsx! {
                    p { class: "groups-note", "Select a surah or juz to see its shared phrases, click a dot under the rub' row, or search a phrase." }
                },
                None => rsx! {},
            }
        }
    }
}
