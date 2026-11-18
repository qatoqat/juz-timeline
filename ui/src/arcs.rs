use crate::band::Band;
use crate::track::EPS;
use dioxus::prelude::*;

const SHOWN: usize = 40;
const SHOWN_PHONE: usize = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum Stroke {
    Phrase,
    Word,
    Twin,
}

/// An echo between two passages of one surah, drawn under the band as an arc between two ayah positions.
/// `twin` names the look-alike it was drawn for, so a click opens exactly that comparison.
#[derive(Clone, PartialEq)]
pub struct Arc {
    pub from: f32,
    pub to: f32,
    pub a: usize,
    pub b: usize,
    pub twin: Option<usize>,
    pub stroke: Stroke,
    pub weight: u8,
    pub tip: String,
}

#[derive(Clone, Copy)]
pub struct Arcs {
    pub arcs: Signal<Vec<Arc>>,
    pub ticks: Signal<Vec<(f32, String)>>,
    pub picked: Signal<Option<(usize, usize, Option<usize>)>>,
}

#[component]
pub fn ArcLane(lo: f32, hi: f32) -> Element {
    let Some(arcs) = try_use_context::<Arcs>() else { return rsx! {} };
    let chosen = try_use_context::<Band>().and_then(|b| (b.chosen)());
    let mut picked = arcs.picked;
    let x = move |p: f32| 100.0 - (p - lo) / (hi - lo) * 100.0;
    let inside = move |p: f32| p >= lo - EPS && p < hi - EPS;
    let mut shown: Vec<Arc> = arcs.arcs.read().iter().filter(|a| inside(a.from) && inside(a.to)).cloned().collect();
    let total = shown.len();
    let touches = |c: &Arc, t: usize| c.a == t || c.b == t;
    let focus = chosen.filter(|&t| shown.iter().any(|c| touches(c, t)));
    shown.sort_by(|a, b| {
        let touch = |c: &Arc| focus.is_some_and(|t| touches(c, t));
        touch(b).cmp(&touch(a)).then(b.weight.cmp(&a.weight))
    });
    shown.truncate(SHOWN);
    let ticks: Vec<(f32, String)> = arcs.ticks.read().iter().filter(|t| inside(t.0)).map(|(p, tip)| (x(*p), tip.clone())).collect();
    if shown.is_empty() && ticks.is_empty() {
        return rsx! {};
    }
    rsx! {
        div { class: "arcs",
            svg { view_box: "0 0 100 100", preserve_aspect_ratio: "none",
                for (n, arc) in shown.iter().cloned().enumerate().rev() {
                    {
                        let (x1, x2) = (x(arc.from), x(arc.to));
                        let depth = (8.0 + (x1 - x2).abs() * 1.6).min(94.0);
                        let d = format!("M{x1} 0 C{x1} {depth} {x2} {depth} {x2} 0");
                        let on = focus.is_some_and(|t| touches(&arc, t));
                        let class = format!(
                            "arc {}{}{}",
                            match arc.stroke { Stroke::Phrase => "phrase", Stroke::Word => "word", Stroke::Twin => "twin" },
                            if focus.is_some() { if on { " on" } else { " dim" } } else { "" },
                            if n >= SHOWN_PHONE { " extra" } else { "" },
                        );
                        let (a, b, twin) = (arc.a, arc.b, arc.twin);
                        rsx! {
                            g { key: "{a}-{b}", class,
                                path { class: "hit", d: d.clone(), "data-tip": "{arc.tip}", onclick: move |_| picked.set(Some((a, b, twin))) }
                                path { class: "line", d }
                            }
                        }
                    }
                }
                for (n, (px, tip)) in ticks.into_iter().enumerate() {
                    path { key: "t{n}", class: "tick", d: "M{px} 0 L{px} 9", "data-tip": "{tip}" }
                }
            }
            p { class: "arc-caption",
                "أصداء محسوبة من اللفظ · computed echoes of wording"
                if total > SHOWN_PHONE { span { class: "phone", " · strongest {SHOWN_PHONE} of {total}" } }
                if total > shown.len() { span { class: "wide", " · strongest {shown.len()} of {total}" } }
            }
        }
    }
}
