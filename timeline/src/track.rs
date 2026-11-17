use crate::data::MANZIL;
use crate::model::Rub;
use crate::labels::{bookmark_tip, fmt_len, juz_tip, rub_tip, sajdah_tip, surah_label, Bookmark, Dome, DIGIT_PX, LABEL_PAD_PX};
use crate::menu::{open_menu, Menu, Target};
use crate::progress::{Mark, Progress};
use crate::{tip, Focus, MUSHAF};
use dioxus::prelude::*;

const MARK_GAP_PX: f32 = 34.0;
pub const EPS: f32 = 1e-3;

#[derive(Clone, Copy, PartialEq)]
enum Line {
    Edge,
    Mid,
}

fn overlaps(a: (f32, f32), b: (f32, f32)) -> bool {
    a.0 < b.1 - EPS && a.1 > b.0 + EPS
}

fn splits(spans: impl Iterator<Item = (f32, f32)>, p: f32) -> bool {
    spans.into_iter().any(|(a, b)| a < p - EPS && b > p + EPS)
}

pub fn span_of(f: Focus) -> (f32, f32) {
    let m = &*MUSHAF;
    match f {
        Focus::Surah(i) => (m.surahs[i].start, m.surahs[i].end),
        Focus::Juz(i) => (m.juz[i].start, m.juz[i].end),
        Focus::Sajdah(i) => (m.sajdah[i].pos, m.sajdah[i].pos + 2.0 * EPS),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Lit {
    Off,
    Half,
    Full,
}

fn lit(focus: Option<Focus>) -> (Vec<Lit>, Vec<Lit>) {
    let m = &*MUSHAF;
    let Some(f) = focus else { return (vec![Lit::Off; 114], vec![Lit::Off; 30]) };
    let span = span_of(f);
    let level = |primary: bool, overlap: bool| match (primary, overlap) {
        (true, _) => Lit::Full,
        (false, true) => Lit::Half,
        _ => Lit::Off,
    };
    let surahs = m.surahs.iter().map(|s| match f {
        Focus::Surah(i) => level(s.num == i + 1, false),
        _ => level(false, overlaps((s.start, s.end), span)),
    });
    let juz = m.juz.iter().map(|j| match f {
        Focus::Juz(i) => level(j.num == i + 1, false),
        _ => level(false, overlaps((j.start, j.end), span)),
    });
    (surahs.collect(), juz.collect())
}

fn line_style(px: f32) -> String {
    if px <= 0.01 {
        "right:0".into()
    } else if px >= 99.99 {
        "left:0".into()
    } else {
        format!("right:calc({px}% - 1px)")
    }
}

fn line_class(kind: Line, dotted: bool) -> String {
    format!("line {}{}", if kind == Line::Edge { "edge" } else { "mid" }, if dotted { " dotted" } else { "" })
}

fn cell_class(base: &str, even: bool, lit: Lit, sel: bool) -> String {
    let lit = match lit {
        Lit::Off => "",
        Lit::Half => " half",
        Lit::Full => " on",
    };
    format!("{base}{}{lit}{}", if even { " even" } else { "" }, if sel { " sel" } else { "" })
}

fn thin<T>(items: impl Iterator<Item = (T, f32)>, min_pct: f32) -> Vec<(T, f32, bool)> {
    let mut last = f32::NEG_INFINITY;
    items
        .map(|(t, px)| {
            let show = px - last >= min_pct;
            if show {
                last = px;
            }
            (t, px, show)
        })
        .collect()
}

#[component]
pub fn Track(lo: f32, hi: f32, focus: Signal<Option<Focus>>, selected: Signal<Option<Focus>>, detail: bool) -> Element {
    let mut width = use_signal(|| 1200.0f32);
    let m = &*MUSHAF;
    let w = width();
    let x = move |p: f32| (p - lo) / (hi - lo) * 100.0;
    let inside = move |p: f32| p >= lo - EPS && p <= hi + EPS;
    let (lit_s, lit_j) = lit(selected().or(focus()));
    let sel = selected();

    let surahs: Vec<(usize, f32, f32)> = m
        .surahs
        .iter()
        .enumerate()
        .filter(|(_, s)| overlaps((s.start, s.end), (lo, hi)))
        .map(|(i, s)| (i, x(s.start.max(lo)), x(s.end.min(hi)) - x(s.start.max(lo))))
        .collect();
    let juz: Vec<(usize, f32, f32)> = m
        .juz
        .iter()
        .enumerate()
        .filter(|(_, j)| overlaps((j.start, j.end), (lo, hi)))
        .map(|(i, j)| (i, x(j.start.max(lo)), x(j.end.min(hi)) - x(j.start.max(lo))))
        .collect();

    let fixed = [m.juz[0].start, m.juz[9].end, m.juz[19].end, m.juz[29].end].map(|p| (p, Line::Edge));
    let mids = [4, 14, 24].map(|k| (m.juz[k].end, Line::Mid));
    let lines: Vec<(String, Line, bool, bool)> = [(lo, Line::Edge), (hi, Line::Edge)]
        .into_iter()
        .chain(fixed.into_iter().chain(mids).filter(|&(p, _)| p > lo + EPS && p < hi - EPS))
        .map(|(p, kind)| {
            let s_split = splits(m.surahs.iter().map(|s| (s.start, s.end)), p);
            let j_split = splits(m.juz.iter().map(|j| (j.start, j.end)), p);
            (line_style(x(p)), kind, s_split, j_split)
        })
        .collect();
    let manzil_lines: Vec<String> = MANZIL
        .iter()
        .map(|&(s, _)| m.surahs[s as usize - 1].start)
        .filter(|&p| p > lo + EPS && p < hi - EPS)
        .map(|p| line_style(x(p)))
        .collect();

    let sajdah: Vec<(usize, f32)> = m
        .sajdah
        .iter()
        .enumerate()
        .filter(|(_, s)| inside(s.pos))
        .map(|(i, s)| (i, x(s.pos)))
        .collect();
    let manzil: Vec<(usize, char, f32)> = MANZIL
        .iter()
        .enumerate()
        .map(|(n, &(s, c))| (n, c, m.surahs[s as usize - 1].start))
        .filter(|&(_, _, p)| p >= lo - EPS && p < hi - EPS)
        .map(|(n, c, p)| (n, c, x(p)))
        .collect();
    let gap = MARK_GAP_PX / w * 100.0;
    let rubs: Vec<&Rub> = m.rub.iter().filter(|r| detail && overlaps((r.pos, r.end), (lo, hi))).collect();
    let rub_cells: Vec<(usize, f32, f32)> =
        rubs.iter().map(|r| (r.index, x(r.pos.max(lo)), x(r.end.min(hi)) - x(r.pos.max(lo)))).collect();
    let aligned = |p: f32| (p - p.round()).abs() < EPS || m.surahs.iter().any(|s| (s.start - p).abs() < EPS);
    let rub_lines: Vec<(String, bool)> = [lo, hi]
        .into_iter()
        .map(|p| (line_style(x(p)), splits(rubs.iter().map(|r| (r.pos, r.end)), p)))
        .chain(rubs.iter().filter(|r| r.pos > lo + EPS).map(|r| (line_style(x(r.pos)), !aligned(r.pos))))
        .collect();
    let page_marks = thin(
        rubs.iter().filter(|r| r.index % 4 == 0 && r.pos > lo + EPS).map(|r| (r.pages().0, x(r.pos))),
        gap * 0.8,
    );
    let first_page = lo.floor() as u32 + 1;
    let last_page = (hi - EPS).ceil() as u32;

    let progress = use_context::<Progress>();
    let done = progress.done.read();
    let edit = (progress.edit)() && detail;
    let menu = use_context::<Signal<Option<Menu>>>();
    let active = (progress.active)();
    let bookmarks: Vec<(usize, u16, f32, String)> = progress
        .bookmarks
        .read()
        .iter()
        .enumerate()
        .map(|(i, &page)| (i, page, page as f32 - 0.5))
        .filter(|&(_, _, p)| !detail && inside(p))
        .map(|(i, page, p)| (i, page, x(p), line_style(x(p))))
        .collect();
    let mark_class = |base: String, m: Mark| if done.has(m) { base + " done" } else { base };

    let ticks: Vec<(String, bool)> = (1..=604i32)
        .filter(|&k| !detail && inside(k as f32 - 0.5) && !progress.bookmarks.read().contains(&(k as u16)))
        .map(|k| (line_style(x(k as f32 - 0.5)), k % 5 == 0))
        .collect();

    let mut enter = move |f: Focus| focus.set(Some(f));
    let mut leave = move || focus.set(None);
    let mut click = move |f: Focus| {
        if !detail {
            selected.set(if selected() == Some(f) { None } else { Some(f) });
        } else if let Some(m) = Mark::of(f).filter(|_| edit) {
            progress.toggle(m);
        }
    };

    rsx! {
        div { class: "scroller",
        div {
            class: if detail { "row" } else { "row main" },
            "data-lo": "{lo}",
            "data-hi": "{hi}",
            onresize: move |e| if let Ok(sz) = e.get_content_box_size() { width.set(sz.width as f32) },
            if !detail {
                div { class: "lane bmk",
                    for (i, page, px, _) in bookmarks.iter().cloned() {
                        b {
                            key: "{i}",
                            class: if i == active { "bm-handle active" } else { "bm-handle" },
                            style: "right:{px}%",
                            "data-bm": "{i}",
                            "data-tip": "{bookmark_tip(page)}",
                            oncontextmenu: move |e| open_menu(menu, &e, Target::Bookmark(i)),
                            Bookmark {}
                        }
                    }
                }
            }
            div { class: "lane sajdah",
                for (style, thick) in ticks {
                    i { class: if thick { "bm-tick thick" } else { "bm-tick" }, style }
                }
                for (_, _, _, style) in bookmarks.iter().cloned() {
                    i { class: "line bm", style }
                }
                for &(i, px) in sajdah.iter() {
                    b {
                        key: "{i}",
                        class: if sel == Some(Focus::Sajdah(i)) { "sj sel" } else { "sj" },
                        style: "right:{px}%",
                        onmouseenter: move |_| enter(Focus::Sajdah(i)),
                        onmouseleave: move |_| leave(),
                        onclick: move |_| click(Focus::Sajdah(i)),
                        oncontextmenu: move |e| open_menu(menu, &e, Target::Sajdah(i)),
                        "data-tip": "{sajdah_tip(i)}",
                        Dome {}
                    }
                }
                for (n, c, px) in manzil.iter().copied() {
                    b {
                        key: "m{n}",
                        class: "mz",
                        style: "right:calc({px}% + 2px)",
                        "data-tip": "Manzil {n + 1} (فمي بشوق) · starts {tip(MANZIL[n].0 as usize - 1)}",
                        "{c}"
                    }
                }
            }
            div { class: "bar surahs",
                for (i, a, sw) in surahs {
                    div {
                        key: "{i}",
                        class: mark_class(cell_class("surah", i % 2 == 1, lit_s[i], sel == Some(Focus::Surah(i))), Mark::Surah(i)),
                        style: "right:{a}%;width:{sw}%",
                        "data-tip": "{tip(i)}",
                        onmouseenter: move |_| enter(Focus::Surah(i)),
                        onmouseleave: move |_| leave(),
                        onclick: move |_| click(Focus::Surah(i)),
                        oncontextmenu: move |e| open_menu(menu, &e, Target::Surah(i)),
                        span { class: "lbl", "{surah_label(i, sw * w / 100.0)}" }
                    }
                }
                for (style, kind, split, _) in lines.iter().cloned() {
                    i { class: line_class(kind, split), style }
                }
                for style in manzil_lines {
                    i { class: "line edge", style }
                }
                for (_, _, _, style) in bookmarks.iter().cloned() {
                    i { class: "line bm", style }
                }
            }
            div { class: "bar",
                for (i, a, jw) in juz {
                    div {
                        key: "{i}",
                        class: mark_class(cell_class("juz", i % 2 == 1, lit_j[i], sel == Some(Focus::Juz(i))), Mark::Juz(i)),
                        style: "right:{a}%;width:{jw}%",
                        onmouseenter: move |_| enter(Focus::Juz(i)),
                        onmouseleave: move |_| leave(),
                        onclick: move |_| click(Focus::Juz(i)),
                        oncontextmenu: move |e| open_menu(menu, &e, Target::Juz(i)),
                        "data-tip": "{juz_tip(i)}",
                        if jw * w / 100.0 >= 2.0 * DIGIT_PX + LABEL_PAD_PX { span { class: "lbl", "{i + 1}" } }
                    }
                }
                for (style, kind, _, split) in lines.iter().cloned() {
                    i { class: line_class(kind, split), style }
                }
                for (_, _, _, style) in bookmarks.iter().cloned() {
                    i { class: "line bm", style }
                }
            }
            if detail {
                div { class: "bar rubs",
                    for (i, a, rw) in rub_cells {
                        div {
                            key: "{i}",
                            class: mark_class(if i % 2 == 1 { "rubc even".into() } else { "rubc".into() }, Mark::Rub(i)),
                            onclick: move |_| if edit { progress.toggle(Mark::Rub(i)) },
                            oncontextmenu: move |e| open_menu(menu, &e, Target::Rub(i)),
                            style: "right:{a}%;width:{rw}%",
                            "data-tip": "{rub_tip(i)}",
                            if i % 4 == 0 && MUSHAF.rub[i].pos >= lo - EPS && rw * w / 100.0 >= 130.0 {
                                span { class: "hz", "hizb {i / 4 + 1}" }
                            }
                            if rw * w / 100.0 >= 26.0 { span { class: "lbl", "{fmt_len(MUSHAF.rub[i].length)}" } }
                        }
                    }
                    for (style, dotted) in rub_lines {
                        i { class: if dotted { "line rl dotted" } else { "line rl" }, style }
                    }
                }
                div { class: "lane pages",
                    span { class: "first", "{first_page}" }
                    for (p, px, show) in page_marks {
                        if show {
                            span { key: "{p}-{px}", style: if px < 0.5 { "right:0".to_string() } else { format!("right:{px}%;transform:translateX(50%)") }, "{p}" }
                        }
                    }
                    span { class: "last", "{last_page}" }
                }
            }
        }
        }
    }
}
