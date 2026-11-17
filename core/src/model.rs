use crate::data::{FIRST_PAGE_LINES, JUZ, PAGES, RUB, RUB_LINES, RUKU, SAJDAH, SURAHS};
use std::sync::LazyLock;

pub const TOTAL_PAGES: f32 = 604.0;

static SURAH_OFFSETS: LazyLock<Vec<u32>> = LazyLock::new(|| {
    let mut acc = 0;
    let mut out = vec![0];
    for s in SURAHS.iter() {
        acc += s.2 as u32;
        out.push(acc);
    }
    out
});

static PAGE_STARTS: LazyLock<Vec<u32>> = LazyLock::new(|| PAGES.iter().map(|&(s, a)| ayah_index(s, a)).collect());

fn ayah_index(surah: u8, ayah: u16) -> u32 {
    SURAH_OFFSETS[(surah as usize - 1).min(SURAHS.len())] + ayah as u32 - 1
}

/// Position measured in Madani-mushaf pages (0.0 = start of page 1, 604.0 = end).
pub fn page_pos(surah: u8, ayah: u16) -> f32 {
    let idx = ayah_index(surah, ayah);
    let starts = &*PAGE_STARTS;
    let p = starts.partition_point(|&s| s <= idx).saturating_sub(1);
    let (lo, hi) = (starts[p], starts[p + 1]);
    p as f32 + (idx - lo) as f32 / (hi - lo).max(1) as f32
}

pub struct Surah {
    pub num: usize,
    pub name: &'static str,
    pub tname: &'static str,
    pub ayat: u16,
    pub makki: bool,
    pub start: f32,
    pub end: f32,
}

pub struct Juz {
    pub num: usize,
    pub start: f32,
    pub end: f32,
    pub from: (u8, u16),
}

pub struct Sajdah {
    pub surah: u8,
    pub ayah: u16,
    pub pos: f32,
}

pub struct Ruku {
    pub index: usize,
    pub surah: u8,
    pub ayah: u16,
    pub pos: f32,
}

pub struct Rub {
    pub index: usize,
    pub from: (u8, u16),
    pub pos: f32,
    pub end: f32,
    pub length: f32,
}

impl Rub {
    pub fn hizb(&self) -> usize {
        self.index / 4 + 1
    }

    pub fn quarter(&self) -> &'static str {
        ["", "¼", "½", "¾"][self.index % 4]
    }

    pub fn pages(&self) -> (u32, u32) {
        (self.pos.floor() as u32 + 1, (self.end - 1e-3).ceil() as u32)
    }
}

const LINES_PER_PAGE: u32 = 15;

fn global_line(page: u16, line: u8) -> u32 {
    let before: u32 = (1..page as u32)
        .map(|p| if p <= 2 { FIRST_PAGE_LINES[p as usize - 1] as u32 } else { LINES_PER_PAGE })
        .sum();
    before + line as u32 - 1
}

fn header_lines(from: u32, to: u32) -> u32 {
    (2..=114u8)
        .filter(|&s| (from + 1..=to).contains(&ayah_index(s, 1)))
        .map(|s| if s == 9 { 1 } else { 2 })
        .sum()
}

fn rub_length(i: usize) -> f32 {
    let (s, a) = RUB[i];
    let (n_s, n_a) = RUB[i + 1];
    let start = global_line(RUB_LINES[i].0, RUB_LINES[i].1);
    let end = if i == 239 { global_line(605, 1) } else { global_line(RUB_LINES[i + 1].0, RUB_LINES[i + 1].1) };
    let lines = end - start - header_lines(ayah_index(s, a), ayah_index(n_s, n_a));
    (lines as f32 / LINES_PER_PAGE as f32 * 2.0).round() / 2.0
}

pub struct Mushaf {
    pub surahs: Vec<Surah>,
    pub juz: Vec<Juz>,
    pub sajdah: Vec<Sajdah>,
    pub rub: Vec<Rub>,
    pub ruku: Vec<Ruku>,
}

impl Mushaf {
    pub fn build() -> Self {
        let surahs = SURAHS
            .iter()
            .enumerate()
            .map(|(i, &(name, tname, ayat, makki))| {
                let n = i as u8 + 1;
                let end = if n == 114 { TOTAL_PAGES } else { page_pos(n + 1, 1) };
                Surah { num: i + 1, name, tname, ayat, makki, start: page_pos(n, 1), end }
            })
            .collect();
        let juz = JUZ
            .windows(2)
            .enumerate()
            .map(|(i, w)| Juz {
                num: i + 1,
                start: page_pos(w[0].0, w[0].1),
                end: if i == 29 { TOTAL_PAGES } else { page_pos(w[1].0, w[1].1) },
                from: w[0],
            })
            .collect();
        let sajdah = SAJDAH
            .iter()
            .map(|&(surah, ayah, _)| Sajdah { surah, ayah, pos: page_pos(surah, ayah) })
            .collect();
        let rub = RUB
            .windows(2)
            .enumerate()
            .map(|(index, w)| Rub {
                index,
                from: w[0],
                pos: page_pos(w[0].0, w[0].1),
                end: if index == 239 { TOTAL_PAGES } else { page_pos(w[1].0, w[1].1) },
                length: rub_length(index),
            })
            .collect();
        let ruku = RUKU
            .iter()
            .enumerate()
            .map(|(index, &(surah, ayah))| Ruku { index, surah, ayah, pos: page_pos(surah, ayah) })
            .collect();
        Mushaf { surahs, juz, sajdah, rub, ruku }
    }
}
