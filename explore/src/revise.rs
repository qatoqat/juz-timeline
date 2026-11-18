use crate::relations::{key_of, Relations, Seg, STORY};
use crate::themes::{open, open_twin, Theme, Themes, View};
use dioxus::prelude::*;
use quran_ui::band::Band;
use quran_ui::overlay::{ayah_pos, range_of, Selection};
use quran_ui::MUSHAF;
use std::collections::BTreeSet;

const CUE_WORDS: usize = 4;
const FOLLOW_WORDS: usize = 5;

/// Where the shared wording first parts: a vowel or و/ف difference in shared words, or a short fork with words on both
/// sides, past the openings and before the last row. None means the passages part only later on.
fn cue(rel: &Relations, k: usize, side: usize, data: Option<&crate::data::Data>) -> Option<Option<(String, String)>> {
    let segs = rel.ladder(k, side, data)?;
    let last = segs.len().saturating_sub(1);
    Some(segs.into_iter().enumerate().find_map(|(n, s)| match s {
        Seg::Same(words) => words.into_iter().find(|w| w.1 != b'.').map(|(this, _, other)| (this, other)),
        Seg::Fork(a, b) if n > 0 && n < last && !a.is_empty() && !b.is_empty() && a.len().max(b.len()) <= CUE_WORDS => Some((a.join(" "), b.join(" "))),
        Seg::Fork(..) => None,
    }))
}

/// A word as read for comparison: waqf signs, tatweel and a first-letter assimilation shadda set aside (as tools/twins.py voc).
fn reading(text: &str) -> String {
    text.split(' ')
        .map(|w| {
            let w: String = w.chars().filter(|&c| !('\u{06D6}'..='\u{06ED}').contains(&c) && c != '\u{0640}' && c != '\u{00A0}').collect();
            let mut chars: Vec<char> = w.chars().collect();
            if chars.len() > 1 && chars[1] == '\u{0651}' {
                chars.remove(1);
            }
            chars.into_iter().collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn surah_of(themes: &[Theme], t: usize) -> &'static str {
    MUSHAF.surahs[themes[t].surah as usize - 1].name
}

#[component]
pub fn Revise(themes: Themes) -> Element {
    let view = use_context::<View>();
    let band = use_context::<Band>();
    let selection = use_context::<Selection>();
    let Some(rel) = (view.rel)() else {
        return rsx! { p { class: "themes-note", if (view.rel_failed)() { "Related passages are not available right now." } else { "Loading related passages…" } } };
    };
    let Some((lo, hi)) = (selection.selected)().map(range_of) else {
        return rsx! { p { class: "themes-note", "Select a surah or juz: its look-alike passages, retold stories and refrains will be listed here to revise together." } };
    };
    let data = (view.data)();
    let list = themes.0.clone();
    let inside = |g: u16| { let p = ayah_pos(key_of(g)); p >= lo && p < hi };
    let mut rows: Vec<(usize, usize, usize)> = rel
        .twins
        .iter()
        .enumerate()
        .filter_map(|(k, tw)| {
            let side = (0..2).find(|&s| inside(tw.sides[s].0))?;
            let matched = tw.flags[side].iter().filter(|&&f| f != b'*').count();
            Some((matched, k, side))
        })
        .collect();
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let stories: Vec<(usize, Vec<usize>)> = (0..list.len())
        .filter(|&t| list[t].start >= lo && list[t].start < hi)
        .filter_map(|t| {
            let partners: Vec<usize> = rel.links_of[t].iter().filter(|&&l| rel.links[l].kind == STORY && rel.links[l].strong).map(|&l| rel.partner(l, t)).collect();
            (!partners.is_empty()).then_some((t, partners))
        })
        .collect();
    let refrains: Vec<usize> = (0..rel.refrains.len()).filter(|&r| rel.refrains[r].at.iter().any(|a| inside(a.0))).collect();

    rsx! {
        div { class: "revise",
            p { class: "tp-sec-head", span { class: "ar", "مقاطع متشابهة" } " · look-alike passages · {rows.len()}" }
            if rows.is_empty() { p { class: "themes-note", "No look-alike passages start in this range." } }
            for (_, k, side) in rows {
                {
                    let tw = &rel.twins[k];
                    let (this, other) = (tw.sides[side], tw.sides[1 - side]);
                    let diff = cue(&rel, k, side, data.as_deref());
                    let (go_list, go_rel) = (list.clone(), rel.clone());
                    rsx! {
                        div {
                            key: "{k}",
                            class: "rv-row",
                            dir: "rtl",
                            onclick: move |_| open_twin(view, band, selection, &go_list, &go_rel, k, side),
                            span { class: "rv-ref", "{rel.range_label(this.0, this.1)}" }
                            span { class: "rv-arrow", "≈" }
                            span { class: "rv-ref other", span { class: "ar", "{MUSHAF.surahs[key_of(other.0).0 as usize - 1].name}" } " {rel.range_label(other.0, other.1)}" }
                            match diff {
                                Some(Some((a, b))) => rsx! { span { class: "rv-diff", span { class: "w diff", "{a}" } " | " span { class: "w diff", "{b}" } } },
                                Some(None) => rsx! { span { class: "rv-diff later", span { class: "ar", "الفرق فيما يلي" } " · differs further on" } },
                                None => rsx! {},
                            }
                        }
                    }
                }
            }
            if !stories.is_empty() {
                p { class: "tp-sec-head", span { class: "ar", "قصص في مواضع أخرى" } " · told elsewhere · {stories.len()}" }
                for (t, partners) in stories {
                    {
                        let surahs: BTreeSet<(usize, usize)> = partners.iter().map(|&p| (list[p].surah as usize, p)).collect();
                        let mut seen = BTreeSet::new();
                        let chips: Vec<(usize, &'static str)> = surahs.into_iter().filter(|(s, _)| seen.insert(*s)).map(|(_, p)| (p, surah_of(&list, p))).collect();
                        let row_list = list.clone();
                        rsx! {
                            div { key: "s{t}", class: "rv-story",
                                span { class: "rv-open", dir: "rtl", "{rel.openings[t]} …" }
                                span { class: "lchips", dir: "rtl",
                                    for (p, name) in chips {
                                        {
                                            let chip_list = row_list.clone();
                                            rsx! { button { key: "{p}", class: "lchip person", onclick: move |_| open(view, band, selection, &chip_list, t, Some(p)), "{name}" } }
                                        }
                                    }
                                }
                                p { class: "len", "{list[t].text}" }
                            }
                        }
                    }
                }
            }
            if !refrains.is_empty() {
                p { class: "tp-sec-head", span { class: "ar", "لوازم" } " · refrains · {refrains.len()}" }
                RefrainChips { refrains }
            }
        }
    }
}

#[component]
pub fn RefrainChips(refrains: Vec<usize>) -> Element {
    let view = use_context::<View>();
    let mut opened = use_signal(|| None::<usize>);
    let Some(rel) = (view.rel)() else { return rsx! {} };
    let data = (view.data)();
    let phrase = |r: usize| -> String {
        let (g, st) = rel.refrains[r].at[0];
        rel.words(g, data.as_deref()).map_or("…".to_string(), |w| w[st..(st + rel.refrains[r].len).min(w.len())].join(" "))
    };
    rsx! {
        div { class: "lchips refrains", dir: "rtl",
            for &r in refrains.iter() {
                button { key: "{r}", class: if opened() == Some(r) { "chip on" } else { "chip" }, onclick: move |_| opened.set(if opened() == Some(r) { None } else { Some(r) }),
                    "{phrase(r)}"
                    span { class: "badge", "×{rel.refrains[r].at.len()}" }
                }
            }
        }
        if let Some(r) = opened().filter(|r| refrains.contains(r)) {
            Refrain { key: "{r}", r }
        }
    }
}

#[component]
fn Refrain(r: usize) -> Element {
    let view = use_context::<View>();
    let Some(rel) = (view.rel)() else { return rsx! {} };
    let data = (view.data)();
    let refrain = &rel.refrains[r];
    let own = |g: u16, st: usize| -> String {
        rel.words(g, data.as_deref()).map_or(String::new(), |w| w[st.min(w.len())..(st + refrain.len).min(w.len())].join(" "))
    };
    let first = reading(&own(refrain.at[0].0, refrain.at[0].1));
    rsx! {
        div { class: "refrain",
            p { class: "hint", dir: "ltr", span { class: "ar", "ما بعدها" } " · what follows each time" }
            for (n, &(g, st)) in refrain.at.iter().enumerate() {
                {
                    let (s, a) = key_of(g);
                    let words = rel.words(g, data.as_deref()).unwrap_or_default();
                    let rest: Vec<String> = words.iter().skip(st + refrain.len).take(FOLLOW_WORDS).cloned().collect();
                    let follow = if rest.is_empty() { rel.next_words(g, data.as_deref()).map(|w| format!("۝ {}", w.join(" "))).unwrap_or_default() } else { rest.join(" ") };
                    let wording = own(g, st);
                    let differs = reading(&wording) != first;
                    rsx! {
                        div { key: "{n}", class: "rf-row", dir: "rtl",
                            span { class: "rv-ref", "{s}:{a}" }
                            span { class: if differs { "rf-own differs" } else { "rf-own" }, "data-tip": differs.then_some("Reads differently here"), "{wording}" }
                            span { class: "rf-follow", "{follow} …" }
                        }
                    }
                }
            }
        }
    }
}
