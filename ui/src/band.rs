use crate::track::EPS;
use dioxus::prelude::*;

const LABEL_CHAR_PX: f32 = 6.5;
const LABEL_PAD_PX: f32 = 14.0;

#[derive(Clone, PartialEq)]
pub struct Cell {
    pub id: usize,
    pub start: f32,
    pub end: f32,
    pub number: usize,
    pub label: String,
    pub tip: String,
    pub heavy: bool,
}

#[derive(Clone, Copy)]
pub struct Band {
    pub cells: Signal<Vec<Cell>>,
    pub hovered: Signal<Option<usize>>,
    pub chosen: Signal<Option<usize>>,
}

fn fit(label: &str, number: usize, px: f32) -> String {
    let room = ((px - LABEL_PAD_PX) / LABEL_CHAR_PX) as usize;
    let num = number.to_string();
    if room < num.len() {
        return String::new();
    }
    if room < num.len() + 6 {
        return num;
    }
    let chars: Vec<char> = label.chars().collect();
    let take = room - num.len() - 2;
    let text: String = if chars.len() > take { chars[..take.saturating_sub(1)].iter().collect::<String>() + "…" } else { label.to_string() };
    format!("{num} {text}")
}

#[component]
pub fn BandLane(lo: f32, hi: f32, width: f32) -> Element {
    let Some(band) = try_use_context::<Band>() else { return rsx! {} };
    let x = move |p: f32| (p - lo) / (hi - lo) * 100.0;
    let (mut hovered, mut chosen) = (band.hovered, band.chosen);
    let (hot, pick) = (hovered(), chosen());
    let cells: Vec<(Cell, f32, f32)> = band
        .cells
        .read()
        .iter()
        .filter(|c| c.end > lo + EPS && c.start < hi - EPS)
        .map(|c| {
            let (a, b) = (x(c.start.max(lo)), x(c.end.min(hi)));
            (c.clone(), a, b - a)
        })
        .collect();

    rsx! {
        div { class: "bar themes", dir: "ltr",
            for (n, (cell, right, w)) in cells.into_iter().enumerate() {
                div {
                    key: "{cell.id}",
                    class: format!(
                        "theme{}{}{}{}",
                        if n % 2 == 1 { " even" } else { "" },
                        if cell.heavy { " heavy" } else { "" },
                        if hot == Some(cell.id) { " hot" } else { "" },
                        if pick == Some(cell.id) { " on" } else { "" },
                    ),
                    style: "right:{right}%;width:{w}%",
                    "data-tip": "{cell.tip}",
                    onmouseenter: move |_| hovered.set(Some(cell.id)),
                    onmouseleave: move |_| hovered.set(None),
                    onclick: move |_| chosen.set(if chosen() == Some(cell.id) { None } else { Some(cell.id) }),
                    span { class: "tl", "{fit(&cell.label, cell.number, w * width / 100.0)}" }
                }
            }
        }
    }
}
