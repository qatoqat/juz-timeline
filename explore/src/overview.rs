use crate::data::Data;
use crate::tree::{ayah_label, light, unlight, Shared};
use dioxus::prelude::*;
use quran_ui::model::TOTAL_PAGES;
use quran_ui::overlay::Overlay;
use quran_ui::MUSHAF;

const PHRASE_WORDS: usize = 4;
const OTHERS_SHOWN: usize = 3;

fn ruku_end(r: usize) -> f32 {
    MUSHAF.ruku.get(r + 1).map_or(TOTAL_PAGES, |n| n.pos)
}

fn number_in_surah(r: usize) -> usize {
    let surah = MUSHAF.ruku[r].surah;
    MUSHAF.ruku[..=r].iter().rev().take_while(|k| k.surah == surah).count()
}

struct Line {
    ayah: usize,
    family: usize,
    phrase: String,
    cue: String,
    others: Vec<usize>,
}

fn lines(d: &Data, r: usize) -> Vec<Line> {
    let (lo, hi) = (MUSHAF.ruku[r].pos, ruku_end(r));
    let mut ayat: Vec<usize> = (0..d.keys.len()).filter(|&a| d.pos[a] >= lo - 1e-4 && d.pos[a] < hi - 1e-4).collect();
    ayat.sort_by_key(|&a| d.keys[a]);
    ayat.into_iter()
        .filter_map(|a| {
            let family = d.default_root(a)?;
            let row = d.rows(family).into_iter().find(|row| row.ayah == a)?;
            let cue = match row.after_cue {
                Some(i) => row.after[i].clone(),
                None if row.after.is_empty() => "۝".to_string(),
                None => row.after.join(" "),
            };
            let others = d.families[family].members.iter().map(|m| m.0).filter(|&m| m != a).collect();
            Some(Line { ayah: a, family, phrase: d.short_phrase(family, PHRASE_WORDS), cue, others })
        })
        .collect()
}

#[component]
pub fn RukuList(data: Shared, lo: f32, hi: f32, pending: Signal<Option<usize>>) -> Element {
    let overlay = use_context::<Overlay>();
    let d = data.0.clone();
    let rukus: Vec<usize> = (0..MUSHAF.ruku.len()).filter(|&r| MUSHAF.ruku[r].pos < hi - 1e-3 && ruku_end(r) > lo + 1e-3).collect();

    rsx! {
        div { class: "ruku-list",
            for r in rukus {
                {
                    let k = &MUSHAF.ruku[r];
                    let items = lines(&d, r);
                    let surah = &MUSHAF.surahs[k.surah as usize - 1];
                    rsx! {
                        div { key: "{r}", class: "ruku-block",
                            p { class: "ruku-title",
                                "Ruku' {number_in_surah(r)} · "
                                span { class: "ar", "{surah.name}" }
                                " {k.surah}:{k.ayah} · page {k.pos.floor() as u32 + 1}"
                                span { class: "count", if items.len() == 1 { " · 1 ayah with look-alikes" } else { " · {items.len()} ayat with look-alikes" } }
                            }
                            for line in items {
                                {
                                    let (a, f) = (line.ayah, line.family);
                                    let key = d.keys[a];
                                    let mut ayat = line.others.clone();
                                    ayat.push(a);
                                    let hover_d = d.clone();
                                    let others: Vec<String> = line.others.iter().take(OTHERS_SHOWN).map(|&o| ayah_label(&d, o)).collect();
                                    let more = line.others.len().saturating_sub(OTHERS_SHOWN);
                                    rsx! {
                                        div {
                                            key: "{a}",
                                            class: "ruku-line",
                                            dir: "rtl",
                                            onmouseenter: move |_| light(overlay, &hover_d, &ayat),
                                            onmouseleave: move |_| unlight(overlay),
                                            onclick: move |_| {
                                                let mut pending = pending;
                                                pending.set(Some(f));
                                                let mut picked = overlay.picked;
                                                picked.set(Some(key));
                                            },
                                            span { class: "ref", "{ayah_label(&d, a)}" }
                                            span { class: "rl-phrase", dir: "rtl", "{line.phrase} " span { class: "w cue", "{line.cue}" } }
                                            span { class: "rl-others",
                                                "↔ {others.join(\" · \")}"
                                                if more > 0 { " +{more}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
