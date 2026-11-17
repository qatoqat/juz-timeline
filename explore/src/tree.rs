use crate::data::{leaves, Data, Node};
use crate::nav::Nav;
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{Overlay, Selection};
use quran_ui::MUSHAF;
use std::rc::Rc;

const ROW: f32 = 40.0;
const GUTTER: f32 = 28.0;

#[derive(Clone)]
pub struct Shared(pub Rc<Data>);

impl PartialEq for Shared {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, Copy)]
pub struct Drill(pub Signal<bool>);

#[derive(Clone, Copy)]
struct Root(usize);

pub fn ayah_label(data: &Data, a: usize) -> String {
    let (s, n) = data.keys[a];
    format!("{s}:{n}")
}

pub fn surah_name(data: &Data, a: usize) -> &'static str {
    MUSHAF.surahs[data.keys[a].0 as usize - 1].name
}

pub fn light(overlay: Overlay, data: &Data, ayat: &[usize]) {
    let mut hot = overlay.hot;
    hot.set(ayat.iter().map(|&a| data.keys[a]).collect());
}

pub fn unlight(overlay: Overlay) {
    let mut hot = overlay.hot;
    hot.set(vec![]);
}

#[component]
fn Gutter(nodes: Vec<Node>, before: bool) -> Element {
    let total = leaves(&nodes) as f32 * ROW;
    let (from, to) = if before { (0.0, GUTTER) } else { (GUTTER, 0.0) };
    let mid = GUTTER / 2.0;
    let mut acc = 0.0;
    let paths: Vec<(String, f32, bool)> = nodes
        .iter()
        .map(|n| {
            let h = n.leaves() as f32 * ROW;
            let cy = acc + h / 2.0;
            acc += h;
            let d = format!("M{from} {} C{mid} {} {mid} {cy} {to} {cy}", total / 2.0, total / 2.0);
            (d, 1.0 + (n.ayat.len() as f32 - 1.0).min(4.0) * 0.4, n.all_late())
        })
        .collect();
    rsx! {
        svg { class: "gutter", width: "{GUTTER}", height: "{total}", view_box: "0 0 {GUTTER} {total}",
            for (d, w, late) in paths {
                path { class: if late { "late" } else { "" }, d, stroke_width: "{w}" }
            }
        }
    }
}

#[component]
fn Branch(node: Node, before: bool, picked: usize) -> Element {
    let Shared(data) = use_context::<Shared>();
    let Drill(drill) = use_context::<Drill>();
    let overlay = use_context::<Overlay>();
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let nav = use_context::<Nav>();
    let Root(family) = use_context::<Root>();
    let mut open = use_signal(|| false);
    let mut revealed = use_signal(|| false);
    let height = node.leaves() as f32 * ROW;
    let hidden = drill() && !before && !revealed();
    let words = match (node.end, before) {
        (true, true) => "ابتداء الآية".to_string(),
        (true, false) => "۝".to_string(),
        _ => node.words.join(" "),
    };
    let rest = node.rest.join(" ");
    let forms_tip = (!node.forms.is_empty()).then(|| format!("Written differently: {}", node.forms.join("  |  ")));
    let refs: Vec<usize> = if node.kids.is_empty() { node.ayat.clone() } else { vec![] };
    let (enter_data, click_data) = (data.clone(), data.clone());
    let (enter_ayat, click_ayat) = (node.ayat.clone(), node.ayat.clone());
    let class = format!(
        "tlabel{}{}{}{}{}{}",
        if node.ayat.contains(&picked) { " path" } else { "" },
        if node.end { " end" } else { "" },
        if node.prefix { " prefix" } else { "" },
        if node.forms.is_empty() { "" } else { " forms" },
        if node.all_late() { " late" } else { "" },
        if hidden { " hidden" } else { "" },
    );

    rsx! {
        div { class: "tnode", dir: if before { "ltr" } else { "rtl" }, style: "--h:{height}px",
            span {
                class,
                onmouseenter: move |_| light(overlay, &enter_data, &enter_ayat),
                onmouseleave: move |_| unlight(overlay),
                onclick: move |_| {
                    if drill() && !before {
                        revealed.set(true);
                    }
                    light(overlay, &click_data, &click_ayat);
                },
                span { class: "words", dir: "rtl", "data-tip": forms_tip, "{words}" if open() { " {rest}" } }
                if !node.rest.is_empty() && !hidden {
                    button {
                        class: "more",
                        "data-tip": if open() { "Show less" } else { "Show the rest of the ayah" },
                        onclick: move |e| {
                            e.stop_propagation();
                            open.toggle();
                        },
                        if open() { "‹" } else { "…" }
                    }
                }
                if node.ayat.len() > 1 { span { class: "badge", "×{node.ayat.len()}" } }
                for a in refs {
                    button {
                        class: format!("ref{}{}", if a == picked { " self" } else { "" }, if node.late.contains(&a) { " late" } else { "" }),
                        "data-tip": if node.late.contains(&a) {
                            format!("{} · {} · joins partway through the phrase", ayah_label(&data, a), surah_name(&data, a))
                        } else {
                            format!("{} · {}", ayah_label(&data, a), surah_name(&data, a))
                        },
                        onclick: {
                            let key = data.keys[a];
                            move |e: MouseEvent| {
                                e.stop_propagation();
                                nav.keep(family);
                                selection.show(key, overlay);
                            }
                        },
                        oncontextmenu: {
                            let (s, n) = data.keys[a];
                            move |e: MouseEvent| open_menu(menu, &e, Target::Ayah(s, n))
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
    use_context_provider(|| Root(family));
    let overlay = use_context::<Overlay>();
    let d = data.0.clone();
    let tree = d.tree(family, picked);
    let phrase = d.phrase(family);
    let ayat: Vec<usize> = d.families[family].members.iter().map(|m| m.0).chain(tree.late.iter().copied()).collect();
    let count = d.families[family].members.len();
    let late = tree.late.len();
    let (enter_d, click_d) = (d.clone(), d.clone());
    let (enter_ayat, click_ayat) = (ayat.clone(), ayat);

    rsx! {
        div { class: "tree-scroll", dir: "rtl",
            div { class: "tree",
                div { class: "side after", dir: "rtl",
                    Gutter { nodes: tree.after.clone(), before: false }
                    div { class: "kids",
                        for (n, node) in tree.after.into_iter().enumerate() {
                            Branch { key: "a{n}", node, before: false, picked }
                        }
                    }
                }
                div {
                    class: "root",
                    onmouseenter: move |_| light(overlay, &enter_d, &enter_ayat),
                    onmouseleave: move |_| unlight(overlay),
                    onclick: move |_| light(overlay, &click_d, &click_ayat),
                    span { class: "words", dir: "rtl", "{phrase}" }
                    span { class: "badge", "×{count}" }
                    if late > 0 { span { class: "badge late", "data-tip": "Ayat joining partway through the phrase", "+{late} late" } }
                }
                div { class: "side before", dir: "ltr",
                    Gutter { nodes: tree.before.clone(), before: true }
                    div { class: "kids",
                        for (n, node) in tree.before.into_iter().enumerate() {
                            Branch { key: "b{n}", node, before: true, picked }
                        }
                    }
                }
            }
        }
    }
}
