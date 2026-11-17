use crate::data::{leaves, Data, Node};
use dioxus::prelude::*;
use quran_ui::overlay::{Overlay, Selection};
use quran_ui::MUSHAF;
use std::rc::Rc;

const ROW: f32 = 34.0;
const GUTTER: f32 = 28.0;

#[derive(Clone)]
pub struct Shared(pub Rc<Data>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

pub fn ayah_label(data: &Data, a: usize) -> String {
    let (s, n) = data.keys[a];
    format!("{s}:{n}")
}

pub fn surah_name(data: &Data, a: usize) -> &'static str {
    MUSHAF.surahs[data.keys[a].0 as usize - 1].name
}

fn hover(overlay: Overlay, data: &Data, ayat: &[usize]) {
    let mut hot = overlay.hot;
    hot.set(ayat.iter().map(|&a| data.keys[a]).collect());
}

fn clear(overlay: Overlay) {
    let mut hot = overlay.hot;
    hot.set(vec![]);
}

#[component]
fn Gutter(nodes: Vec<Node>, before: bool) -> Element {
    let total = leaves(&nodes) as f32 * ROW;
    let (from, to) = if before { (0.0, GUTTER) } else { (GUTTER, 0.0) };
    let mid = GUTTER / 2.0;
    let mut acc = 0.0;
    let paths: Vec<(String, f32)> = nodes
        .iter()
        .map(|n| {
            let h = n.leaves() as f32 * ROW;
            let cy = acc + h / 2.0;
            acc += h;
            let d = format!("M{from} {} C{mid} {} {mid} {cy} {to} {cy}", total / 2.0, total / 2.0);
            (d, 1.0 + (n.ayat.len() as f32 - 1.0).min(4.0) * 0.4)
        })
        .collect();
    rsx! {
        svg { class: "gutter", width: "{GUTTER}", height: "{total}", view_box: "0 0 {GUTTER} {total}",
            for (d, w) in paths {
                path { d, stroke_width: "{w}" }
            }
        }
    }
}

#[component]
fn Branch(node: Node, before: bool, picked: usize) -> Element {
    let Shared(data) = use_context::<Shared>();
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let height = node.leaves() as f32 * ROW;
    let on_path = node.ayat.contains(&picked);
    let words = match (node.end, before) {
        (true, true) => "ابتداء الآية".to_string(),
        (true, false) => "۝".to_string(),
        _ => node.words.join(" "),
    };
    let refs: Vec<usize> = if node.kids.is_empty() { node.ayat.clone() } else { vec![] };
    let ayat = node.ayat.clone();
    let hover_data = data.clone();

    rsx! {
        div { class: "tnode", dir: if before { "ltr" } else { "rtl" }, style: "height:{height}px",
            span {
                class: format!("tlabel{}{}", if on_path { " path" } else { "" }, if node.end { " end" } else { "" }),
                onmouseenter: move |_| hover(overlay, &hover_data, &ayat),
                onmouseleave: move |_| clear(overlay),
                span { class: "words", dir: "rtl", "{words}" if node.more { " …" } }
                if node.ayat.len() > 1 { span { class: "badge", "×{node.ayat.len()}" } }
                for a in refs {
                    button {
                        class: if a == picked { "ref self" } else { "ref" },
                        "data-tip": "{ayah_label(&data, a)} · {surah_name(&data, a)}",
                        onclick: {
                            let key = data.keys[a];
                            move |_| selection.show(key, overlay)
                        },
                        "{ayah_label(&data, a)}"
                    }
                }
            }
            if !node.kids.is_empty() {
                Gutter { nodes: node.kids.clone(), before }
                div { class: "kids",
                    for (n, kid) in node.kids.iter().cloned().enumerate() {
                        Branch { key: "{n}", node: kid, before, picked }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Tree(data: Shared, family: usize, picked: usize) -> Element {
    use_context_provider(|| data.clone());
    let overlay = use_context::<Overlay>();
    let d = data.0.clone();
    let (after, before) = d.tree(family);
    let phrase = d.phrase(family);
    let ayat: Vec<usize> = d.families[family].members.iter().map(|m| m.0).collect();
    let count = ayat.len();

    rsx! {
        div { class: "tree-scroll", dir: "rtl",
            div { class: "tree",
                div { class: "side", dir: "rtl",
                    Gutter { nodes: after.clone(), before: false }
                    div { class: "kids",
                        for (n, node) in after.into_iter().enumerate() {
                            Branch { key: "a{n}", node, before: false, picked }
                        }
                    }
                }
                div {
                    class: "root",
                    onmouseenter: move |_| hover(overlay, &d, &ayat),
                    onmouseleave: move |_| clear(overlay),
                    span { class: "words", dir: "rtl", "{phrase}" }
                    span { class: "badge", "×{count}" }
                }
                div { class: "side", dir: "ltr",
                    Gutter { nodes: before.clone(), before: true }
                    div { class: "kids",
                        for (n, node) in before.into_iter().enumerate() {
                            Branch { key: "b{n}", node, before: true, picked }
                        }
                    }
                }
            }
        }
    }
}
