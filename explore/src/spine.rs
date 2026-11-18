use crate::tree::{ayah_label, light, surah_name, unlight, written_tip, Shared};
use dioxus::prelude::*;
use quran_ui::menu::{open_menu, Menu, Target};
use quran_ui::overlay::{Overlay, Selection};

const SPINE_PAD: usize = 2;
/// A stretch this long that no shared phrase comes near folds into one "…" (about a mushaf line).
const FOLD_MIN: usize = 8;

pub fn plural(n: usize, word: &str) -> String {
    if n == 1 { format!("1 {word}") } else { format!("{n} {word}s") }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cell {
    Word(usize),
    /// Words start..end, folded.
    Fold(usize, usize),
    /// Folds the words that follow it back from `start`.
    Unfold(usize),
}

/// The row's words from 2 before the first phrase to 2 after the last, one cell each, except that a long stretch no
/// phrase comes near is a single fold unless `open` holds its first word. Phrases are (start, end) word spans.
pub fn row(spans: &[(usize, usize)], total: usize, open: &[usize]) -> (usize, usize, Vec<Cell>) {
    let lo = spans.iter().map(|s| s.0).min().unwrap_or(0).saturating_sub(SPINE_PAD);
    let hi = (spans.iter().map(|s| s.1).max().unwrap_or(total) + SPINE_PAD).min(total);
    let near = |i: usize| spans.is_empty() || spans.iter().any(|&(s, e)| i + SPINE_PAD >= s && i < e + SPINE_PAD);
    let mut cells = vec![];
    let mut i = lo;
    while i < hi {
        let end = (i..hi).find(|&j| near(j)).unwrap_or(hi);
        if end - i < FOLD_MIN {
            cells.push(Cell::Word(i));
            i += 1;
            continue;
        }
        if open.contains(&i) {
            cells.push(Cell::Unfold(i));
            cells.extend((i..end).map(Cell::Word));
        } else {
            cells.push(Cell::Fold(i, end));
        }
        i = end;
    }
    (lo, hi, cells)
}

/// The grid column (from 1) of a word that has a cell of its own.
pub fn column(cells: &[Cell], word: usize) -> Option<usize> {
    cells.iter().position(|&c| c == Cell::Word(word)).map(|n| n + 1)
}

#[component]
pub fn Spine(data: Shared, ayah: usize, root: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let menu = use_context::<Signal<Option<Menu>>>();
    // (ayah, first word) of every fold opened on this page.
    let mut opened = use_signal(Vec::<(usize, usize)>::new);
    let d = data.0.clone();
    let words = d.words(ayah);
    let (ks, ka) = d.keys[ayah];
    let current = root();
    let spellings = current.filter(|f| d.family_of[ayah].contains(f)).map(|f| d.tree(f, ayah).spellings).unwrap_or_default();
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
    // Laid out from every phrase of the ayah, not only the bars drawn, so the row stays put when another bar is picked.
    let all: Vec<(usize, usize)> = d.family_of[ayah].iter().map(|&f| d.span_in(f, ayah)).collect();
    let open: Vec<usize> = opened().iter().filter(|o| o.0 == ayah).map(|o| o.1).collect();
    let total = words.len();
    let (lo, hi, cells) = row(&all, total, &open);
    let bars: Vec<(usize, usize, usize, usize)> = bars
        .into_iter()
        .filter_map(|(f, s, e, layer)| Some((f, column(&cells, s)?, column(&cells, e - 1)? + 1, layer)))
        .collect();
    // (key, grid column, class, text, tip, the fold it opens or closes)
    let shown: Vec<(String, usize, &str, String, Option<String>, Option<usize>)> = cells
        .into_iter()
        .enumerate()
        .map(|(n, c)| match c {
            Cell::Word(i) => {
                let tip = spellings.get(&i).map(|f| written_tip(f));
                (format!("w{i}"), n + 1, if tip.is_some() { "sw odd" } else { "sw" }, words[i].to_string(), tip, None)
            }
            Cell::Fold(s, e) => (format!("f{s}"), n + 1, "sfold", "…".to_string(), Some(format!("Show {}", plural(e - s, "more word"))), Some(s)),
            Cell::Unfold(s) => (format!("u{s}"), n + 1, "sfold", "‹".to_string(), Some("Fold these words again".to_string()), Some(s)),
        })
        .collect();
    let columns = shown.len();

    rsx! {
        div { class: "spine-wrap",
            div { class: "spine-head",
                span {
                    class: "here",
                    "data-tip": "Right-click for quran.com and bookmarks",
                    oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(ks, ka)),
                    "{ayah_label(&d, ayah)} · "
                    span { class: "ar", "{surah_name(&d, ayah)}" }
                }
                span { " · {plural(d.family_of[ayah].len(), \"shared phrase\")}" }
                if bars.len() > 1 { span { class: "hint", " · tap a bar to branch from that phrase" } }
            }
            div { class: "spine", dir: "rtl",
                if lo > 0 { span { class: "ellipsis", "…" } }
                div { class: "spine-grid", style: "grid-template-columns: repeat({columns}, auto)",
                    for (key, col, class, w, tip, fold) in shown {
                        span {
                            key: "{key}",
                            class,
                            "data-tip": tip,
                            style: "grid-row:1;grid-column:{col}",
                            onclick: move |_| {
                                if let Some(s) = fold {
                                    let mut list = opened.write();
                                    match list.iter().position(|&o| o == (ayah, s)) {
                                        Some(n) => _ = list.remove(n),
                                        None => list.push((ayah, s)),
                                    }
                                }
                            },
                            "{w}"
                        }
                    }
                    for (f, s, e, layer) in bars {
                        div {
                            key: "f{f}",
                            class: if current == Some(f) { "sbar on" } else { "sbar" },
                            style: "grid-row:{layer + 2};grid-column:{s} / {e}",
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
    let menu = use_context::<Signal<Option<Menu>>>();
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
                            oncontextmenu: {
                                let (s, n) = d.keys[a];
                                move |e: MouseEvent| open_menu(menu, &e, Target::Ayah(s, n))
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
