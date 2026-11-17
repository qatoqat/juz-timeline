use crate::concept::{ConceptMap, Current};
use crate::ontology::{parse_text, Ontology, SharedOntology};
use crate::spine::plural;
use crate::spread::{Occurrences, Spread, Texts};
use dioxus::prelude::*;
use quran_ui::overlay::{ayah_pos, Overlay, Spot};
use quran_ui::MUSHAF;
use std::collections::HashMap;
use std::rc::Rc;

const QUL: &str = "https://qul.tarteel.ai/resources/ayah-topics";
const CORPUS: &str = "https://corpus.quran.com/ontology.jsp";
const FETCH: &str = r#"fetch("topics.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const FETCH_TEXT: &str = r#"fetch("topic-ayat.json").then(r => r.text()).then(t => dioxus.send(t))"#;
const SEARCH_LIMIT: usize = 30;
const TIP_NAMES: usize = 3;

fn dots(o: &Ontology) -> Vec<Spot> {
    let rukus: std::collections::HashSet<(u8, u16)> = MUSHAF.ruku.iter().filter(|r| r.ayah != 1).map(|r| (r.surah, r.ayah)).collect();
    let sajdahs: HashMap<(u8, u16), bool> = MUSHAF.sajdah.iter().map(|sj| ((sj.surah, sj.ayah), sj.listed)).collect();
    (0..o.keys.len() as u16)
        .map(|a| {
            let key = o.key(a);
            let here = o.of_ayah.get(&a).map_or(&[][..], |v| v.as_slice());
            let mut tip = format!("{}:{} · ", key.0, key.1);
            if here.is_empty() {
                tip += "no concepts";
            } else {
                let names: Vec<&str> = here.iter().take(TIP_NAMES).map(|&i| o.concepts[i].english.as_str()).collect();
                tip += &names.join(", ");
                if here.len() > TIP_NAMES {
                    tip += &format!(" +{}", here.len() - TIP_NAMES);
                }
            }
            let sajdah = sajdahs.get(&key).copied();
            Spot { key, pos: ayah_pos(key), tip, weight: here.len().min(3) as u8, ruku: rukus.contains(&key), sajdah: sajdah.is_some(), unlisted: sajdah == Some(false) }
        })
        .collect()
}

#[derive(Clone, Copy)]
struct History {
    list: Signal<Vec<usize>>,
    cursor: Signal<usize>,
    restoring: Signal<bool>,
}

impl History {
    fn record(mut self, i: usize) {
        if *self.restoring.peek() {
            self.restoring.set(false);
            return;
        }
        let mut list = self.list.peek().clone();
        let cursor = *self.cursor.peek();
        if list.get(cursor) == Some(&i) {
            return;
        }
        list.truncate(if list.is_empty() { 0 } else { cursor + 1 });
        list.push(i);
        self.cursor.set(list.len() - 1);
        self.list.set(list);
    }

    fn can(self, delta: i32) -> bool {
        let target = (self.cursor)() as i32 + delta;
        target >= 0 && (target as usize) < self.list.read().len()
    }

    fn go(mut self, delta: i32, mut topic: Signal<Option<usize>>) {
        if self.can(delta) {
            let target = (*self.cursor.peek() as i32 + delta) as usize;
            self.cursor.set(target);
            self.restoring.set(true);
            topic.set(Some(self.list.peek()[target]));
        }
    }
}

#[component]
pub fn TopicsPage() -> Element {
    let mut onto = use_signal(|| None::<Rc<Ontology>>);
    let mut texts = use_signal(|| None::<Rc<HashMap<u16, String>>>);
    let overlay = use_context_provider(|| Overlay {
        dots: Signal::new(vec![]),
        marks: Signal::new(vec![]),
        picked: Signal::new(quran_ui::hash::get("ayah").and_then(|v| quran_ui::hash::parse_ayah(&v))),
        hot: Signal::new(vec![]),
    });
    let current = use_context_provider(|| Current { topic: Signal::new(None), rollup: Signal::new(true) });
    let history = use_context_provider(|| History { list: Signal::new(vec![]), cursor: Signal::new(0), restoring: Signal::new(false) });
    use_future(move || async move {
        let mut fetch = document::eval(FETCH);
        if let Ok(text) = fetch.recv::<String>().await {
            if let Some(o) = Ontology::parse(&text) {
                let mut dots_signal = overlay.dots;
                dots_signal.set(dots(&o));
                let wanted = quran_ui::hash::get("topic").and_then(|v| v.parse::<u32>().ok()).and_then(|id| o.by_id(id));
                onto.set(Some(Rc::new(o)));
                let mut topic = current.topic;
                topic.set(wanted);
            }
        }
    });
    let mut text_started = use_signal(|| false);
    use_effect(move || {
        if (current.topic)().is_some() && !*text_started.peek() {
            text_started.set(true);
            spawn(async move {
                let mut fetch = document::eval(FETCH_TEXT);
                if let Ok(json) = fetch.recv::<String>().await {
                    texts.set(parse_text(&json).map(Rc::new));
                }
            });
        }
    });
    use_effect(move || {
        let topic = (current.topic)();
        let rollup = (current.rollup)();
        let Some(o) = onto() else { return };
        let mut hot = overlay.hot;
        hot.set(vec![]);
        let mut marks = overlay.marks;
        marks.set(topic.map_or(vec![], |i| {
            o.ayat(i, rollup).iter().map(|&a| {
                let key = o.key(a);
                Spot { key, pos: ayah_pos(key), tip: format!("{}:{} · {}", key.0, key.1, o.concepts[i].english), weight: 3, ..Spot::default() }
            }).collect()
        }));
        quran_ui::hash::set(&[("topic", topic.map(|i| o.concepts[i].id.to_string()))]);
        if let Some(i) = topic {
            history.record(i);
        }
    });
    use_effect(move || quran_ui::hash::set(&[("ayah", (overlay.picked)().map(|(s, a)| format!("{s}:{a}")))]));

    rsx! {
        style { {include_str!("../assets/similar.css")} }
        style { {include_str!("../assets/topics.css")} }
        quran_ui::Page {
            ruku: false,
            title: rsx! {
                h1 {
                    "المفاهيم"
                    span { class: "sub", "Concepts" }
                    a { class: "explore", href: "../", "Explore" }
                    a { class: "explore", href: "{CORPUS}", target: "_blank", rel: "noopener", "Source ↗" }
                }
            },
            extra: rsx! { Panel { onto: onto().map(SharedOntology), texts: Texts(texts()) } },
        }
    }
}

#[component]
fn Chips(onto: SharedOntology, items: Vec<usize>, head: Option<String>) -> Element {
    let Current { mut topic, .. } = use_context::<Current>();
    let o = onto.0.clone();
    rsx! {
        if let Some(h) = head { p { class: "chips-head", "{h}" } }
        div { class: "chips", dir: "rtl",
            for i in items {
                button {
                    key: "{i}",
                    class: if topic() == Some(i) { "chip on" } else { "chip" },
                    "data-tip": "{o.concepts[i].english} · {plural(o.ayat(i, true).len(), \"ayah\")}",
                    onclick: move |_| topic.set(Some(i)),
                    "{o.title(i)}"
                    span { class: "badge", "{o.ayat(i, true).len()}" }
                }
            }
        }
    }
}

#[component]
fn Browse(onto: SharedOntology) -> Element {
    let o = onto.0.clone();
    rsx! {
        p { class: "groups-note", "Pick a concept, search above, or click an ayah dot to see its concepts." }
        div { class: "browse",
            for r in o.roots() {
                div { key: "{r}", class: "browse-group",
                    Chips { onto: onto.clone(), items: vec![r], head: None }
                    if !o.concepts[r].kids.is_empty() {
                        Chips { onto: onto.clone(), items: o.concepts[r].kids.clone(), head: None }
                    }
                }
            }
        }
    }
}

#[component]
fn Panel(onto: Option<SharedOntology>, texts: Texts) -> Element {
    let overlay = use_context::<Overlay>();
    let current = use_context::<Current>();
    let history = use_context::<History>();
    let mut query = use_signal(String::new);
    let by_surah = use_signal(|| false);
    let Some(shared) = onto else { return rsx! { p { class: "groups-note", "Loading concepts…" } } };
    let o = shared.0.clone();
    let searching = query().trim().chars().count() >= 2;
    let results = if searching { o.search(&query(), SEARCH_LIMIT) } else { vec![] };
    let picked = (overlay.picked)();
    let here: Option<Vec<usize>> = picked.and_then(|key| o.slot(key)).map(|a| o.of_ayah.get(&a).cloned().unwrap_or_default());
    let topic = (current.topic)().filter(|&i| i < o.concepts.len());
    let mut topic_signal = current.topic;

    rsx! {
        div { class: "similar topics",
            div { class: "finder",
                input {
                    class: "phrase-search",
                    r#type: "search",
                    placeholder: "ابحث عن مفهوم · search a concept",
                    value: "{query}",
                    oninput: move |e| query.set(e.value()),
                }
                if searching && results.is_empty() { span { class: "groups-note", "No concept matches." } }
            }
            if searching && !results.is_empty() {
                Chips { onto: shared.clone(), items: results, head: None }
            }
            if let (Some((s, a)), Some(list)) = (picked, here) {
                if list.is_empty() {
                    p { class: "chips-head", "No concepts are tagged in {s}:{a}." }
                } else {
                    Chips { onto: shared.clone(), items: list, head: Some(format!("Concepts in {s}:{a}")) }
                }
            }
            match topic {
                Some(i) => rsx! {
                    div { key: "{i}", class: "topic-view",
                        div { class: "picked-bar",
                            div { class: "navbar",
                                button { class: "nav", disabled: !history.can(-1), onclick: move |_| history.go(-1, topic_signal), "‹ Back" }
                                button { class: "nav", disabled: !history.can(1), onclick: move |_| history.go(1, topic_signal), "Forward ›" }
                            }
                            button { class: "nav", onclick: move |_| topic_signal.set(None), "All concepts" }
                        }
                        ConceptMap { onto: shared.clone(), i }
                        Spread { onto: shared.clone(), i, by_surah }
                        Occurrences { onto: shared.clone(), i, texts }
                    }
                },
                None if !searching => rsx! { Browse { onto: shared.clone() } },
                None => rsx! {},
            }
            p { class: "credit",
                "Concepts from the "
                a { href: "{CORPUS}", target: "_blank", rel: "noopener", "Quranic Arabic Corpus ontology ↗" }
                " (GNU GPL), via "
                a { href: "{QUL}", target: "_blank", rel: "noopener", "QUL ↗" }
                ". Ayah text: Quran Foundation."
            }
        }
    }
}
