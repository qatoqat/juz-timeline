use crate::compare::member_in;
use crate::data::Data;
use crate::relations::{key_of, Relations, ECHO};
use crate::themes::{describe, Theme, View};
use dioxus::prelude::*;
use quran_ui::arcs::{Arc, Arcs, Stroke};
use quran_ui::band::Band;
use quran_ui::overlay::{ayah_pos, AyahKey, Overlay, Spot};
use quran_ui::MUSHAF;
use std::collections::{BTreeMap, BTreeSet};

const TWIN_WEIGHT: u8 = 20;

fn pos(g: u16) -> f32 {
    ayah_pos(key_of(g))
}

fn label(themes: &[Theme], a: usize, b: usize, what: &str) -> String {
    let (x, y) = (&themes[a], &themes[b]);
    format!("{}:{}–{} ↔ {}–{} · {what}", x.surah, x.from, x.to, y.from, y.to)
}

fn merge(mut ranges: Vec<(u16, u16)>) -> Vec<(u16, u16)> {
    ranges.sort();
    let mut out: Vec<(u16, u16)> = vec![];
    for r in ranges {
        match out.last_mut() {
            Some(last) if r.0 <= last.1 => last.1 = last.1.max(r.1),
            _ => out.push(r),
        }
    }
    out
}

fn twin_arc(rel: &Relations, themes: &[Theme], k: usize, what: &str) -> Option<Arc> {
    let (a, b) = match rel.twin_target(k, 0)? {
        (a, Some(b)) => (a, b),
        _ => return None,
    };
    let s = rel.twins[k].sides;
    Some(Arc { from: pos(s[0].0), to: pos(s[1].0), a, b, twin: Some(k), stroke: Stroke::Twin, weight: TWIN_WEIGHT, tip: label(themes, a, b, what) })
}

/// The look-alikes of a frame (one surah repeating a passage 3+ times), joined neighbour to neighbour; a stretch that no
/// neighbour shares a look-alike with keeps its own.
fn frame_arcs(rel: &Relations, themes: &[Theme], ks: &[usize]) -> Vec<Arc> {
    let what = "look-alike passages repeated through the surah";
    let stretches = merge(ks.iter().flat_map(|&k| rel.twins[k].sides).collect());
    let at = |r: (u16, u16)| stretches.iter().position(|s| s.0 <= r.0 && r.0 <= s.1).unwrap_or(0);
    let ends = |k: usize| { let s = rel.twins[k].sides; let (x, y) = (at(s[0]), at(s[1])); (x.min(y), x.max(y)) };
    let mut joined = vec![false; stretches.len()];
    let mut picked = vec![];
    for w in 1..stretches.len() {
        if let Some(&k) = ks.iter().find(|&&k| ends(k) == (w - 1, w)) {
            picked.push(k);
            joined[w - 1] = true;
            joined[w] = true;
        }
    }
    for &k in ks {
        let (x, y) = ends(k);
        if !picked.contains(&k) && (!joined[x] || !joined[y]) {
            picked.push(k);
            joined[x] = true;
            joined[y] = true;
        }
    }
    picked.into_iter().filter_map(|k| twin_arc(rel, themes, k, what)).collect()
}

/// Echo arcs inside each surah: look-alikes first, then echo links anchored on their evidence. Phrase arcs wait for
/// similar.json, which holds where each phrase sits.
fn arcs_of(rel: &Relations, themes: &[Theme], data: Option<&Data>) -> Vec<Arc> {
    let mut out = vec![];
    let mut frames: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (k, tw) in rel.twins.iter().enumerate() {
        if key_of(tw.sides[0].0).0 != key_of(tw.sides[1].0).0 {
            continue;
        }
        if tw.frame >= 0 {
            frames.entry(tw.frame).or_default().push(k);
        } else {
            out.extend(twin_arc(rel, themes, k, "look-alike passage"));
        }
    }
    for ks in frames.values() {
        out.extend(frame_arcs(rel, themes, ks));
    }
    let paired: BTreeSet<(usize, usize)> = out.iter().map(|a| (a.a.min(a.b), a.a.max(a.b))).collect();
    for l in rel.links.iter().filter(|l| l.kind == ECHO && l.twins.is_empty() && !paired.contains(&(l.i.min(l.j), l.i.max(l.j)))) {
        let (from, to, stroke) = match (l.families.first(), data, l.words.first()) {
            (Some(&f), Some(d), _) if f < d.families.len() => {
                (d.pos[member_in(d, f, rel.spans[l.i])], d.pos[member_in(d, f, rel.spans[l.j])], Stroke::Phrase)
            }
            (None, _, Some(&(a, _, b, _))) => (pos(a), pos(b), Stroke::Word),
            _ => continue,
        };
        let what = if stroke == Stroke::Word { "rare word" } else { "shared phrase" };
        out.push(Arc { from, to, a: l.i, b: l.j, twin: None, stroke, weight: l.score, tip: label(themes, l.i, l.j, what) });
    }
    out
}

fn spot(key: AyahKey, tip: String) -> Spot {
    Spot { key, pos: ayah_pos(key), tip, weight: 3, ..Spot::default() }
}

/// Timeline lanes of the Ayah themes page: partner marks, sajdah and evidence dots, echo arcs and refrain ticks.
pub fn use_lanes(view: View, band: Band, overlay: Overlay, arcs: Arcs) {
    use_effect(move || {
        let (chosen, list, rel) = ((band.chosen)(), (view.themes)(), (view.rel)());
        let mut marks = overlay.marks;
        let (Some(t), Some(list), Some(rel)) = (chosen, list, rel) else { return marks.set(vec![]) };
        marks.set(
            rel.listed(t)
                .into_iter()
                .flat_map(|(_, p)| {
                    let (lo, hi) = rel.spans[p];
                    let tip = describe(&list[p]);
                    (lo..=hi).map(move |g| { let key = key_of(g); spot(key, format!("{}:{} · {tip}", key.0, key.1)) })
                })
                .collect(),
        );
    });
    use_effect(move || {
        let (chosen, with, twin, rel) = ((band.chosen)(), (view.with)(), (view.twin)(), (view.rel)());
        let mut keys: BTreeSet<AyahKey> = MUSHAF.sajdah.iter().map(|sj| (sj.surah, sj.ayah)).collect();
        let mut dots: Vec<Spot> = MUSHAF
            .sajdah
            .iter()
            .filter(|_| quran_ui::SHOW_SAJDAH())
            .map(|sj| Spot { key: (sj.surah, sj.ayah), pos: sj.pos, tip: format!("{}:{} · sajdah", sj.surah, sj.ayah), sajdah: true, unlisted: !sj.listed, ..Spot::default() })
            .collect();
        if let (Some(t), Some(rel)) = (chosen, rel) {
            let mut shown: Vec<usize> = with.map_or(vec![], |w| rel.twins_for(t, Some(w)).into_iter().map(|(k, _)| k).collect());
            shown.extend(twin.filter(|&k| rel.twins_for(t, None).iter().any(|x| x.0 == k)));
            for k in shown {
                for (lo, hi) in rel.twins[k].sides {
                    for g in lo..=hi {
                        let key = key_of(g);
                        if keys.insert(key) {
                            dots.push(spot(key, format!("{}:{} · look-alike", key.0, key.1)));
                        }
                    }
                }
            }
        }
        let mut dots_signal = overlay.dots;
        dots_signal.set(dots);
    });
    use_effect(move || {
        let (list, rel, data) = ((view.themes)(), (view.rel)(), (view.data)());
        let (Some(list), Some(rel)) = (list, rel) else { return };
        let (mut arcs_signal, mut ticks) = (arcs.arcs, arcs.ticks);
        arcs_signal.set(arcs_of(&rel, &list, data.as_deref()));
        ticks.set(
            rel.refrains
                .iter()
                .flat_map(|r| {
                    let (g, st) = r.at[0];
                    let phrase = rel.words(g, data.as_deref()).map(|w| w[st.min(w.len())..(st + r.len).min(w.len())].join(" ")).unwrap_or_default();
                    let tip = format!("refrain ×{} {phrase}", r.at.len());
                    r.at.iter().map(move |a| (pos(a.0), tip.clone()))
                })
                .collect(),
        );
    });
}
