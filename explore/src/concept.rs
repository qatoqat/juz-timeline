use crate::ontology::{pieces, Edge, Ontology, Piece, SharedOntology};
use crate::spine::plural;
use dioxus::prelude::*;
use quran_ui::overlay::Overlay;

const ROW: f32 = 34.0;
const GUTTER: f32 = 36.0;
const KIDS_SHOWN: usize = 12;
const NAMED_SHOWN: usize = 8;
const SHARED_SHOWN: usize = 6;

#[derive(Clone, Copy)]
pub struct Current {
    pub topic: Signal<Option<usize>>,
    pub rollup: Signal<bool>,
}

#[derive(Clone, PartialEq)]
enum Row {
    Node(usize, &'static str, Vec<u16>, String),
    Head(&'static str),
    More(usize),
}

fn hot(overlay: Overlay, o: &Ontology, ayat: &[u16]) {
    let mut hot = overlay.hot;
    hot.set(ayat.iter().map(|&a| o.key(a)).collect());
}

fn right_rows(o: &Ontology, i: usize, all_kids: bool) -> Vec<Row> {
    let c = &o.concepts[i];
    let mut rows = vec![];
    let shown = if all_kids { c.kids.len() } else { KIDS_SHOWN.min(c.kids.len()) };
    for &k in &c.kids[..shown] {
        let ayat = o.ayat(k, true).to_vec();
        let badge = format!("{}", ayat.len());
        rows.push(Row::Node(k, "kid", ayat, badge));
    }
    if shown < c.kids.len() {
        rows.push(Row::More(c.kids.len() - shown));
    }
    let mut named: Vec<usize> = c.edges.iter().filter(|e| matches!(e.1, Edge::Named | Edge::Related)).map(|e| e.0).collect();
    named.sort_by_key(|&j| std::cmp::Reverse(o.concepts[j].ayat.len()));
    named.dedup();
    if !named.is_empty() {
        rows.push(Row::Head("Linked in the corpus"));
        for &j in named.iter().take(NAMED_SHOWN) {
            let shared = o.shared(i, j);
            let ayat = if shared.is_empty() { o.concepts[j].ayat.clone() } else { shared };
            rows.push(Row::Node(j, "named", ayat, format!("{}", o.concepts[j].ayat.len())));
        }
    }
    let mut together: Vec<(usize, u16)> = c.edges.iter().filter_map(|e| if let Edge::Shared(n) = e.1 { Some((e.0, n)) } else { None }).collect();
    together.sort_by_key(|&(j, n)| (std::cmp::Reverse(n), j));
    if !together.is_empty() {
        rows.push(Row::Head("Appears with"));
        for &(j, n) in together.iter().take(SHARED_SHOWN) {
            rows.push(Row::Node(j, "shared", o.shared(i, j), format!("{n} shared")));
        }
    }
    rows
}

#[component]
fn Gutter(height: f32, from: f32, to: Vec<(f32, bool)>, card_left: bool) -> Element {
    let (x0, x1) = if card_left { (0.0, GUTTER) } else { (GUTTER, 0.0) };
    let mid = GUTTER / 2.0;
    rsx! {
        svg { class: "cgutter", width: "{GUTTER}", height: "{height}", view_box: "0 0 {GUTTER} {height}",
            for (y, dashed) in to {
                path { class: if dashed { "dashed" } else { "" }, d: "M{x0} {from} C{mid} {from} {mid} {y} {x1} {y}" }
            }
        }
    }
}

#[component]
fn Node(onto: SharedOntology, i: usize, kind: &'static str, ayat: Vec<u16>, badge: String) -> Element {
    let Current { mut topic, .. } = use_context::<Current>();
    let overlay = use_context::<Overlay>();
    let o = onto.0.clone();
    let c = &o.concepts[i];
    let tip = if kind == "shared" { format!("{} · {} together", c.english, plural(ayat.len(), "ayah")) } else { c.english.clone() };
    rsx! {
        button {
            class: "cnode {kind}",
            "data-tip": "{tip}",
            onmouseenter: move |_| hot(overlay, &onto.0, &ayat),
            onmouseleave: move |_| {
                let mut hot = overlay.hot;
                hot.set(vec![]);
            },
            onclick: move |_| topic.set(Some(i)),
            if !c.arabic.is_empty() { span { class: "ar", "{c.arabic}" } }
            span { class: "en", "{c.english}" }
            span { class: "badge", "{badge}" }
        }
    }
}

#[component]
pub fn ConceptMap(onto: SharedOntology, i: usize) -> Element {
    let current = use_context::<Current>();
    let mut rollup = current.rollup;
    let mut topic = current.topic;
    let mut all_kids = use_signal(|| false);
    let o = onto.0.clone();
    let c = &o.concepts[i];
    let lineage = o.lineage(i);
    let right = right_rows(&o, i, all_kids());
    let height = (lineage.len().max(right.len()).max(3)) as f32 * ROW;
    let center = |n: usize, k: usize| (height - n as f32 * ROW) / 2.0 + (k as f32 + 0.5) * ROW;
    let mid = height / 2.0;
    let up: Vec<(f32, bool)> = lineage.len().checked_sub(1).map(|k| vec![(center(lineage.len(), k), false)]).unwrap_or_default();
    let down: Vec<(f32, bool)> = right
        .iter()
        .enumerate()
        .filter_map(|(k, r)| match r {
            Row::Node(_, kind, ..) => Some((center(right.len(), k), *kind == "shared")),
            _ => None,
        })
        .collect();
    let total = o.ayat(i, true).len();
    let direct = c.ayat.len();
    let wiki = c.wiki.clone();

    rsx! {
        div { class: "cmap", dir: "rtl",
            div { class: "ccol anc", style: "height:{height}px",
                for &a in lineage.iter() {
                    div { key: "a{a}", class: "crow",
                        Node { onto: onto.clone(), i: a, kind: "anc", ayat: o.ayat(a, true).to_vec(), badge: format!("{}", o.ayat(a, true).len()) }
                    }
                }
            }
            if !up.is_empty() { Gutter { height, from: mid, to: up, card_left: true } } else { div { class: "cgap" } }
            div { class: "ccard",
                div { class: "ctitle",
                    span { class: "ar", "{o.title(i)}" }
                    if !c.arabic.is_empty() { span { class: "en", "{c.english}" } }
                }
                div { class: "ccount",
                    if c.kids.is_empty() || !rollup() {
                        "{plural(direct, \"ayah\")}"
                    } else {
                        "{plural(total, \"ayah\")} with sub-concepts · {direct} directly"
                    }
                }
                if !c.kids.is_empty() {
                    button {
                        class: if rollup() { "drill on" } else { "drill" },
                        "data-tip": "Count and mark the ayat of every sub-concept too",
                        onclick: move |_| rollup.toggle(),
                        "Include sub-concepts"
                    }
                }
            }
            if !down.is_empty() { Gutter { height, from: mid, to: down, card_left: false } } else { div { class: "cgap" } }
            div { class: "ccol kids", style: "height:{height}px",
                for (k, row) in right.into_iter().enumerate() {
                    div { key: "r{k}", class: "crow",
                        match row {
                            Row::Node(j, kind, ayat, badge) => rsx! { Node { onto: onto.clone(), i: j, kind, ayat, badge } },
                            Row::Head(text) => rsx! { span { class: "chead", "{text}" } },
                            Row::More(n) => rsx! { button { class: "more", onclick: move |_| all_kids.set(true), "+{n} more" } },
                        }
                    }
                }
            }
        }
        p { class: "cdesc",
            for (n, piece) in pieces(&c.description).into_iter().enumerate() {
                match piece {
                    Piece::Text(t) => rsx! { span { key: "{n}", "{t}" } },
                    Piece::Link(j, label) if j < o.concepts.len() => rsx! {
                        button { key: "{n}", class: "clink", onclick: move |_| topic.set(Some(j)), "{label}" }
                    },
                    Piece::Link(_, label) => rsx! { span { key: "{n}", "{label}" } },
                }
            }
            if !wiki.is_empty() {
                " "
                a { class: "source", href: "https://en.wikipedia.org/wiki/{wiki}", target: "_blank", rel: "noopener", "Wikipedia ↗" }
            }
        }
    }
}
