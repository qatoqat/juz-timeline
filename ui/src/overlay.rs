use crate::labels::Dome;
use crate::menu::{open_menu, Menu, Target};
use crate::track::EPS;
use crate::Focus;
use dioxus::prelude::*;

pub type AyahKey = (u8, u16);

#[derive(Clone, PartialEq, Default)]
pub struct Spot {
    pub key: AyahKey,
    pub pos: f32,
    pub tip: String,
    pub weight: u8,
    pub ruku: bool,
    pub sajdah: bool,
    pub unlisted: bool,
}

const SPOT_MAX: f32 = 15.0;
const RUKU_MIN: f32 = 11.0;

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

pub fn range_of(focus: Focus) -> (f32, f32) {
    let m = &*crate::MUSHAF;
    match focus {
        Focus::Surah(i) => (m.surahs[i].start, m.surahs[i].end),
        Focus::Juz(i) => (m.juz[i].start, m.juz[i].end),
        Focus::Sajdah(i) => {
            let s = &m.surahs[m.sajdah[i].surah as usize - 1];
            (s.start, s.end)
        }
    }
}

pub fn ayah_pos(key: AyahKey) -> f32 {
    quran_core::model::page_pos(key.0, key.1)
}

#[component]
pub fn OverlayLane(lo: f32, hi: f32, detail: bool) -> Element {
    let Some(overlay) = try_use_context::<Overlay>() else { return rsx! {} };
    let selection = use_context::<Selection>();
    let menu = use_context::<Signal<Option<Menu>>>();
    let x = move |p: f32| (p - lo) / (hi - lo) * 100.0;
    let inside = move |p: f32| p >= lo - EPS && p < hi - EPS;
    let picked = (overlay.picked)();
    let marked: Vec<AyahKey> = overlay.marks.read().iter().map(|s| s.key).collect();
    let hot = overlay.hot.read().clone();
    let source: Vec<Spot> = if detail { overlay.dots.read().clone() } else { overlay.marks.read().clone() };
    let mut spots: Vec<(Spot, f32)> = source.into_iter().filter(|s| inside(s.pos)).map(|s| { let px = x(s.pos); (s, px) }).collect();
    spots.sort_by(|a, b| a.1.total_cmp(&b.1));
    let size = if detail { SPOT_MAX } else { 8.0 };

    rsx! {
        div { class: if detail { "lane dots" } else { "lane marks" },
            for (spot, px) in spots {
                {
                    let (s, a) = spot.key;
                    rsx! {
                        b {
                            key: "{s}:{a}",
                            class: format!(
                                "spot w{}{}{}{}{}{}{}",
                                spot.weight.min(3),
                                if spot.sajdah { " sajdah" } else { "" },
                                if spot.ruku { " ruku" } else { "" },
                                if spot.unlisted { " unlisted" } else { "" },
                                if picked == Some(spot.key) { " picked" } else { "" },
                                if detail && marked.contains(&spot.key) { " marked" } else { "" },
                                if hot.contains(&spot.key) { " hot" } else { "" },
                            ),
                            style: "right:{px}%;width:{size}px;height:{size}px;font-size:{size * 0.62}px",
                            "data-tip": "{spot.tip}",
                            onclick: move |_| {
                                if detail {
                                    let mut picked = overlay.picked;
                                    picked.set(Some(spot.key));
                                } else {
                                    selection.show(spot.key, overlay);
                                }
                            },
                            oncontextmenu: move |e| open_menu(menu, &e, Target::Ayah(s, a)),
                            if spot.sajdah { Dome {} }
                            if spot.ruku && size >= RUKU_MIN { i { class: "rk-in", "ع" } }
                        }
                    }
                }
            }
        }
    }
}
