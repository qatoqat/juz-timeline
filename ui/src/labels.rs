use crate::track::EPS;
use crate::MUSHAF;
use dioxus::prelude::*;

pub const DIGIT_PX: f32 = 7.0;
pub const LABEL_PAD_PX: f32 = 5.0;

pub fn surah_label(i: usize, px: f32) -> String {
    let s = &MUSHAF.surahs[i];
    let digits = s.num.to_string().len() as f32 * DIGIT_PX;
    if px >= digits + s.name.chars().count() as f32 * DIGIT_PX + 3.0 * LABEL_PAD_PX {
        format!("{} {}", s.num, s.name)
    } else if px >= digits + LABEL_PAD_PX {
        s.num.to_string()
    } else {
        String::new()
    }
}

pub fn juz_tip(i: usize) -> String {
    let j = &MUSHAF.juz[i];
    format!("Juz {} · from {}:{} · pages {}–{}", j.num, j.from.0, j.from.1, j.start.floor() as u32 + 1, (j.end - EPS).ceil() as u32)
}

pub fn sajdah_tip(i: usize) -> String {
    let sj = &MUSHAF.sajdah[i];
    let note = if sj.listed { "" } else { " · not in quran.com's list" };
    format!("Sajdah {} of {} · {}:{} · page {}{note}", i + 1, MUSHAF.sajdah.len(), sj.surah, sj.ayah, sj.pos.floor() as u32 + 1)
}

pub fn fmt_len(n: f32) -> String {
    if n.fract() == 0.0 { format!("{n:.0}") } else { format!("{n:.1}") }
}

pub fn rub_tip(i: usize) -> String {
    let r = &MUSHAF.rub[i];
    let (a, b) = r.pages();
    format!(
        "Hizb {} {} · from {}:{} · pages {a}–{b} · about {} pages of text",
        r.hizb(), r.quarter(), r.from.0, r.from.1, fmt_len(r.length)
    )
}

pub fn ruku_tip(i: usize) -> String {
    let r = &MUSHAF.ruku[i];
    format!("Ruku' {} · {}:{} · page {}", i + 1, r.surah, r.ayah, r.pos.floor() as u32 + 1)
}

pub fn bookmark_tip(page: u16) -> String {
    let pos = page as f32 - 0.5;
    let m = &*MUSHAF;
    let surah = m.surahs.iter().rev().find(|s| s.start <= pos).map_or(String::new(), |s| format!("{}\u{1F}{}", s.name, s.num));
    let juz = m.juz.iter().position(|j| pos < j.end).map_or(30, |j| j + 1);
    format!("Reading page {page} · Juz {juz}\t{surah}")
}

#[component]
pub fn Dome() -> Element {
    rsx! {
        svg { view_box: "0 0 16 16", width: "15", height: "15",
            path {
                d: "M1.5 14.5h13M3.6 14.5C2.6 12 2.6 9.6 4.6 8C6.2 6.8 7.6 5.8 8 3.6C8.4 5.8 9.8 6.8 11.4 8C13.4 9.6 13.4 12 12.4 14.5M8 3.6V1.5",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.4",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

#[component]
pub fn Bookmark() -> Element {
    rsx! {
        svg { view_box: "0 0 12 16", width: "11", height: "15",
            path { d: "M2 1h8v14l-4-3.2L2 15z", fill: "currentColor" }
        }
    }
}
