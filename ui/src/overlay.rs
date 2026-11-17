use crate::track::EPS;
use crate::Focus;
use dioxus::prelude::*;

pub type AyahKey = (u8, u16);

#[derive(Clone, PartialEq)]
pub struct Spot {
    pub key: AyahKey,
    pub pos: f32,
    pub tip: String,
    pub weight: u8,
}

#[derive(Clone, Copy)]
pub struct Overlay {
    pub dots: Signal<Vec<Spot>>,
    pub marks: Signal<Vec<Spot>>,
    pub picked: Signal<Option<AyahKey>>,
    pub hot: Signal<Vec<AyahKey>>,
}

#[derive(Clone, Copy)]
pub struct Selection {
    pub focus: Signal<Option<Focus>>,
    pub selected: Signal<Option<Focus>>,
}

impl Selection {
    pub fn show(mut self, key: AyahKey, overlay: Overlay) {
        self.selected.set(Some(Focus::Surah(key.0 as usize - 1)));
        let mut picked = overlay.picked;
        picked.set(Some(key));
    }
}

pub fn ayah_pos(key: AyahKey) -> f32 {
    quran_core::model::page_pos(key.0, key.1)
}

#[component]
pub fn OverlayLane(lo: f32, hi: f32, detail: bool) -> Element {
    let Some(overlay) = try_use_context::<Overlay>() else { return rsx! {} };
    let selection = use_context::<Selection>();
    let x = move |p: f32| (p - lo) / (hi - lo) * 100.0;
    let inside = move |p: f32| p >= lo - EPS && p < hi - EPS;
    let picked = (overlay.picked)();
    let marked: Vec<AyahKey> = overlay.marks.read().iter().map(|s| s.key).collect();
    let hot = overlay.hot.read().clone();
    let spots: Vec<Spot> = if detail { overlay.dots.read().clone() } else { overlay.marks.read().clone() };
    let spots: Vec<(Spot, f32)> = spots.into_iter().filter(|s| inside(s.pos)).map(|s| { let px = x(s.pos); (s, px) }).collect();

    rsx! {
        div { class: if detail { "lane dots" } else { "lane marks" },
            for (spot, px) in spots {
                b {
                    key: "{spot.key.0}:{spot.key.1}",
                    class: format!(
                        "spot w{}{}{}{}",
                        spot.weight.min(3),
                        if picked == Some(spot.key) { " picked" } else { "" },
                        if detail && marked.contains(&spot.key) { " marked" } else { "" },
                        if hot.contains(&spot.key) { " hot" } else { "" },
                    ),
                    style: "right:{px}%",
                    "data-tip": "{spot.tip}",
                    onclick: move |_| {
                        if detail {
                            let mut picked = overlay.picked;
                            picked.set(Some(spot.key));
                        } else {
                            selection.show(spot.key, overlay);
                        }
                    },
                }
            }
        }
    }
}
